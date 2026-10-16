---
phase: 31-rules-tab-on-the-skin
reviewed: 2026-10-16T12:00:00Z
depth: standard
files_reviewed: 15
files_reviewed_list:
  - src/filter/structure.rs
  - src/overlay/app.rs
  - src/overlay/apply.rs
  - src/overlay/body.rs
  - src/overlay/config_view.rs
  - src/overlay/rule_list.rs
  - src/overlay/rules.rs
  - src/overlay/screenshot.rs
  - src/overlay/widgets.rs
  - src/skin/editor.rs
  - src/skin/icons.rs
  - src/skin/mod.rs
  - src/skin/textures.rs
  - src/skin/widgets.rs
  - tests/filter_structure.rs
findings:
  critical: 0
  warning: 8
  info: 9
  total: 17
status: issues_found
---

# Phase 31: Code Review Report

**Reviewed:** 2026-10-16
**Depth:** standard
**Files Reviewed:** 15
**Status:** issues_found

## Summary

I reviewed the phase diff (`d2dd7f4^..HEAD`) for the 15 files, read the code around each hunk,
and ran `cargo test --lib --test filter_structure`: 1178 + 8 pass.

I also tested `Document::insert_rule_at_top` against 12 edge-case inputs in a separate scratch
crate. These included CRLF, files without a trailing newline, a BOM, economy-at-top with rules
after it, a preamble with no blank line before the markers, blank-only and comment-only files, and
mixed line endings. In every case the result was correct:
- The new rule never lands inside the economy section or a `# ff:group` run.
- CRLF line endings are kept.
- The BOM stays at byte 0.
- All other bytes are unchanged.

`remove_block` restores the original bytes except in the two cases listed under IN-02.

The `cfg(test)` light-skin fallback in `skin::of` / `textures::load` is compiled only into
unit-test builds. Integration tests and release builds use the full `Skin::load`, so it cannot
leak into release.

I found no correctness blockers. The main problems are:
- A unit test whose name says CRLF but which no longer contains a CR.
- Several presentation regressions from moving to skin widgets: the 44pt number box clips
  4-digit values, the dropdown cannot scroll, log sliders ignore the arrow keys, and the
  collapsed-row summary runs under the flags and arrows.
- A new hazard: an empty `Show` rule is now inserted as the first block of the file.

## Warnings

### WR-01: `insert_top_crlf` test no longer contains a single CR

**File:** `src/filter/structure.rs:763-784`
**Issue:** The test strings were rewritten with real line breaks instead of `\r\n` / `\n` escape
sequences. `cat -A` on the file shows that no line has a `\r`. Both cases (`p("Hide\n")` and
`p("Show\nHide")`) are LF-only, so the test called `insert_top_crlf` checks LF behaviour and
proves nothing about CRLF. The previous `append_after_trailing_comment_and_edge_inputs` test did
cover `"Show\r\nHide"`, so the CRLF path of the new insertion code has lost its focused test.

The only remaining CRLF coverage is the multi-step `crlf_is_preserved` test. The new branch that
takes the line ending from the first line when inserting at line 0
(`self.lines.first().map(|l| l.eol())`) has no direct test.

The same rewrite turned the other `insert_top_*` tests into multi-line literals. Those are
equivalent but harder to read; for example, a literal tab now stands in for `\t` in
`insert_top_above_disabled_block`.

I checked the behaviour in a scratch crate and it is correct today: `"Hide\r\n"` becomes
`"Show\r\n\r\nHide\r\n"`. The problem is that a future regression would not be caught.

**Fix:** Restore escaped literals:
```rust
let mut d = p("Hide\r\n");
d.insert_rule_at_top(v(&["Show"])).unwrap();
assert_eq!(s(&d), "Show\r\n\r\nHide\r\n");
let mut d = p("Show\r\nHide");
d.insert_rule_at_top(v(&["Show"])).unwrap();
assert_eq!(s(&d), "Show\r\n\r\nShow\r\nHide");
```

### WR-02: + Add rule puts a match-everything `Show` first, shadowing the entire filter until conditions are added

**File:** `src/overlay/rules.rs:1097`, `src/filter/structure.rs:205-274`
**Issue:** The new block is a bare `Show` with no conditions, and it is now the first block of the
file. The filter is first-match-wins and the block has no `Continue`, so every item matches it:
- Every Hide rule stops working.
- Every styled rule stops working.
- The whole economy section stops working.

Before this phase the empty rule was appended just above the economy section, so it only
shadowed that section. Nothing guards the new case:
- The grammar gate passes an empty `Show`.
- Apply writes it.
- The Quit popup's "Apply & quit" writes it.
- The in-game reload then applies it.

One stray click on + Add rule followed by Apply turns the user's filter into "show everything,
default style", with no warning. The spec chose top-of-file insertion, but not this side effect.

**Fix:** Pick one:
- Flag an enabled, condition-less, non-`Continue` rule that sits above other rules as a grammar
  warning (shown, not blocking), e.g. "matches every item; the rules below never apply".
- Insert the new rule disabled (`# Show`) until it has a condition.
- Have Apply confirm when such a rule exists.

### WR-03: 44pt number field cannot show 4-digit values

**File:** `src/overlay/widgets.rs:220` (`NUMBER_FIELD_W = 44.0`), `src/skin/widgets.rs:1046` (`rect.shrink2(vec2(10.0, 3.0))`)
**Issue:** The field is 44pt wide, but `ff_text_field_ex` takes 10pt from each side, leaving 24pt
for text. The test `number_field_is_44_wide_and_keeps_focus_id` confirms this
(`rect.width() + 20.0 == 44.0`).

At regular 15pt, four digits are about 32pt wide. Several slider ranges reach 4 digits:
- `BaseArmour`, `BaseEvasion`, `BaseEnergyShield`: 0-1000
- `StackSize`: 1-5000

Hand-written values can be larger still (e.g. `BaseArmour > 1500`). These values are cut off and
the TextEdit scrolls inside its box. The "exact number" field, the only place that shows an
out-of-range hand-written value exactly, cannot show it.

**Fix:** Size the field to the widest value it can display, e.g.
`max(44, text_width(max(range_max, |current|)) + 20)`. Or use a 64pt field for keywords whose
range is above 999, and keep 44 for the rest.

### WR-04: Log-scale sliders ignore the arrow keys

**File:** `src/overlay/widgets.rs:265-270`
**Issue:** For `log == true`, `themed_slider` passes `step = 0.0` to `ff_slider`. Left/Right then
compute `snap_value(*value - 0.0, ..)`, which returns the same value, so the arrow keys do
nothing on StackSize and the three base-defence sliders. The egui slider this replaced handled
the arrow keys on logarithmic sliders. Home and End still work.

**Fix:** Give the log slider a nonzero step in slider space (e.g. `ln(1+max-min)/100`). Or have
`themed_slider` handle Left/Right itself as ±1 on the integer value when the response has focus.

### WR-05: Skin dropdown has no scroll area; long option lists run off the window

**File:** `src/skin/widgets.rs` `ff_dropdown_ex` popup body; used by `src/overlay/body.rs` `style_picker`, `colour_combo`, minimap shape/size, and the operator dropdowns
**Issue:** Each option takes 26pt and the popup grows without limit. The style picker lists
Custom, the 12 built-ins, and every custom style in `styles.json`; custom styles are unlimited.
The popup can therefore grow taller than the work area:
- egui's popup constrain can only move it, so the bottom entries cannot be reached.
- Near the bottom of the docked window, even 12 colours or 12 shapes can be cut off.

The `egui::ComboBox` code this replaced scrolled, and the old `colour_combo_tests` that checked
scroll-bar clearance were deleted rather than ported.

**Fix:** Wrap the option rows in `ff_scroll_area` with a maximum height (e.g. `min(12 * 26, available screen height)`),
as `picker_popup` already does with `PICKER_MAX_H`.

### WR-06: Collapsed-row summary is painted under the flags and reorder arrows

**File:** `src/overlay/rule_list.rs:482-493` (summary width `inner.max.x - (index_right + 8.0)`)
**Issue:** The right-side child uses `Align::Center` across the full 52pt row, so:
- the 16pt arrows sit at y=18..34 of the row;
- the 18pt flags sit at y=17..35;
- the summary line is centred at y=38 and, at 15pt, spans roughly y=29..47.

The summary is cut only at the right edge of the row, not at the flags or arrows. A long summary
(for example many classes plus a BaseType count plus a level condition) is painted through the
flag icons and arrow art. The 102-rule screenshot hides this only because none of its summaries
is long enough.

**Fix:** Cut the summary at `used_left - 8.0`, as the name already is. Alternatively, place the
flags and arrows on line 1 (`line1_y`) so line 2 can use the full width.

### WR-07: Quoted-list / int-list add field ignores its hint and always says "Type a name, press Enter"

**File:** `src/overlay/widgets.rs:661-676` (`add_field(.., _hint)`)
**Issue:** The `hint` parameter is now ignored (`_hint`) and the placeholder is hard-coded. Callers
pass specific hints:
- `chip_hint` gives "Add a base type…" or "Add a mod…".
- `ints_control` passes "Add a whole number…".

The integer list now asks the user to "Type a name". That is wrong and misleading, since a non-number is
rejected.

**Fix:** Pass `hint` through: `ff_text_field_id(ui, field_id, &mut draft, hint, width)`. If the
"press Enter" wording is wanted, update the callers' hint strings to include it.

### WR-08: Reorder arrows lost their padded 24x24 click target

**File:** `src/overlay/rule_list.rs:272-273`, `src/overlay/widgets.rs:187` (`arrow_button`), `src/skin/widgets.rs:954-961` (`ff_arrow`: 16x16)
**Issue:** The old `arrow_button` and `circular_toggle` padded their hit area to 24x24, which was
documented in the code as a "UI-SPEC carry-over". `ff_arrow` senses only its 16x16 rect, and the
two arrows sit 4pt apart. In a 52pt row the targets are now small enough that a slightly-off click
falls on the row's own click sink and expands or collapses the row instead of moving it.

**Fix:** Inside `ff_arrow`, `interact` on `rect.expand(4.0)` with a derived id, as the removed
`arrow_button` did, or allocate 24x24 and paint the 16pt art centred in it.

## Info

### IN-01: A title comment directly above the first block moves below the new rule

**File:** `src/filter/structure.rs:280-283`
**Issue:** `top_insertion_line` returns `first.span.start`, which includes the leading comments
attached to that block. For `"# My filter v3\nShow\n"`, the result is
`"Show\n\n# My filter v3\nShow\n"`. The file's title or credits comment now sits between rules 1
and 2. This matches the doc comment and is reversible, but users of hand-written filters may not
expect it.
**Fix:** Document this in the + Add rule tooltip or CLAUDE.md, or stop above the first block's
`# ff:` meta lines only and leave ordinary prose comments in place.

### IN-02: Removing a just-added rule leaves a blank line in two layouts

**File:** `src/filter/structure.rs:248-250`
**Issue:** When the insertion point follows a non-blank, non-block line, a separating blank line
is added *before* the new rule. Two layouts trigger this:
- a comment directly above the economy BEGIN marker;
- a comment-only file with no final newline (`"# x"`).

`remove_block` removes only the span plus the blank lines *after* it, so add-then-delete leaves
an extra blank line (`"# note\n\n{BEGIN}…"`). This is harmless, but it is a byte difference.
**Fix:** Accept it, or have `remove_block` also eat one blank line before the span when the line
before that blank is non-blank and was not blank originally. The former is simpler.

### IN-03: The search box's "Clear" text overlays the query text

**File:** `src/overlay/rule_list.rs:702-723`
**Issue:** The Clear hit area covers the last 52pt of the TextEdit and is registered after it.
Typed text that reaches that area is painted under "Clear", and clicks there no longer place the
text cursor.
**Fix:** When the query is non-empty, shrink the field width by the Clear width instead of
overlaying it.

### IN-04: `chip_list` lost its height cap; `_max_height` is dead

**File:** `src/overlay/widgets.rs:548-551`
**Issue:** The ScrollArea was removed and the parameter renamed `_max_height`, so a long integer
list grows the expanded row without limit. The caller in `body.rs:2248` still passes 160.0.
**Fix:** Wrap the list in `ff_scroll_area` capped at `max_height`, or remove the parameter.

### IN-05: Doc comment of `sync_working_styles` is now attached to `nth_row_id`

**File:** `src/overlay/app.rs:232-238`
**Issue:** `nth_row_id` was inserted between the doc comment of `sync_working_styles` and the
function itself. The "Point the rules at the CONFIG tab's…" text now documents `nth_row_id`
(merged with its own line), and `sync_working_styles` has no doc. In addition,
`expand_seeds_the_nth_row` calls a `#[cfg(debug_assertions)]` function from a test that is not
gated, so `cargo test --release` would fail to compile.
**Fix:** Move `nth_row_id` and its doc above the `sync_working_styles` doc block, and gate the
test with `#[cfg(debug_assertions)]`.

### IN-06: Rule rows have no accessible name

**File:** `src/overlay/rule_list.rs:518`
**Issue:** The index, keyword, name and summary are painted directly, and the row's click response
gets no `widget_info`. Assistive tech sees an unlabeled clickable area. The old `Label` carried
`row.label`.
**Fix:** `row_resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, &row.label));`

### IN-07: Locked (style-owned) action labels are no longer greyed; Continue label is not clickable

**File:** `src/overlay/body.rs:2827-2850`, `src/overlay/body.rs:1247-1264`
**Issue:**
- `card_header` → `ff_field_row` always paints the label in `pal::LABEL`. The old code used
  `DISABLED_TEXT` when `lock.is_some()`.
- The "Continue to later rules" text is a plain label next to `ff_check`. Clicking the text no
  longer toggles it, as it did with `egui::Checkbox`.

**Fix:** Add a label-colour option to `ff_field_row`, or a disabled flag. Make the Continue label
sense clicks and toggle the check.

### IN-08: Light test skin changes 9-slice geometry in unit tests

**File:** `src/skin/textures.rs:25-49`, `src/skin/mod.rs:60-61`
**Issue:** Any unit test that reaches `skin::of` without `install` gets 2x2 textures.
Slice-dependent layout, such as the cap widths in `three_h_rects(.., art.left.size(), ..)` and
`ff_check`'s aspect ratio, then differs from production. Layout assertions in those tests do not
reflect the real art. Release is unaffected. In addition, if `Skin::load` panics inside
`load_light`, `LIGHT` stays `true` for that test thread.
**Fix:** Make the 2x2 stand-ins the real art's dimensions (`ColorImage::new(real_size, ..)`
decoded from the PNG header only), and reset `LIGHT` with a drop guard.

### IN-09: `--expand N` past the last rule is silently ignored

**File:** `src/overlay/app.rs:831-837`
**Issue:** A screenshot run with an out-of-range `--expand` renders a collapsed list without
saying so, which makes scripted screenshot checks misleading.
**Fix:** `eprintln!` a warning, or fail the screenshot run, when `nth_row_id` returns `None`.

---

_Reviewed: 2026-10-16_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
