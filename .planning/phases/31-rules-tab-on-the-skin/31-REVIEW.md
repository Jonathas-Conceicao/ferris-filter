---
phase: 31-rules-tab-on-the-skin
reviewed: 2026-10-16T15:00:00Z
depth: standard
iteration: 2
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
  warning: 2
  info: 12
  total: 14
status: issues_found
---

# Phase 31: Code Review Report (iteration 2)

**Reviewed:** 2026-10-16
**Depth:** standard
**Files Reviewed:** 15
**Status:** issues_found

## Summary

This is a re-review of the seven fix commits `179c0ef..d84cb0e` (`git diff b7521f6..HEAD -- src tests`),
read against the code around each change. `cargo test --lib --test filter_structure` passes
(1184 + 8).

Fix verification:

| Finding | Result |
|---|---|
| WR-01 CRLF test | Fixed. The literals are escaped again (`\r\n`). A new mixed-ending case covers the first-line EOL rule. |
| WR-03 number field width | Fixed for committed values. The slider reserve and the field width agree for values the range can produce. A side effect is listed as IN-01. |
| WR-04 log-slider arrows | Fixed for in-range values. **It introduces a new bug for out-of-range values (WR-01 below).** |
| WR-05 dropdown scroll | Fixed. `room` uses the larger of the space above and below the box. egui 0.36's `Popup` tries the symmetric alignment (`TOP_START`) before `MENU_ALIGNS`, so a popup sized for the space above is placed above. The frame margin (12) and gap (2) are subtracted. |
| WR-06 summary cut | Fixed. The summary width now uses `used_left - 8.0`, the same bound as the name. |
| WR-07 add-field hint | Fixed. `hint` is passed through, and the caller strings were updated. |
| WR-08 arrow hit target | Fixed. The arrows are registered after the row's click sink, so the padded target is on top. With 3pt vertical padding, the row's blank-area click under the arrows (x=500, y=38) still expands the row. |
| WR-02 (top insertion) | Skipped as an author decision. Kept below as an accepted warning, not a blocker. |

I found no blockers. The 9 Info items from iteration 1 were out of the fixer's scope and are all
still present. They are carried below as IN-04..IN-12.

## Warnings

### WR-01: Log-slider Right arrow lowers an out-of-range hand-written value (and Left raises one below the minimum)

**File:** `src/overlay/widgets.rs:310-324`
**Issue:** The new WR-04 branch computes
`new = (value.clamp(min, max) + delta).clamp(min, max)` and writes it whenever `new != *value`.

Take a hand-written value above the range, for example `BaseArmour >= 1500` (range 0-1000, log).
When the slider has keyboard focus (Tab), pressing **Right** gives `clamp(1500) = 1000`, then
`1001`, then `1000`. That is `!= 1500`, so the value is rewritten to **1000** and the card emits an
edit: the key meant "increase" lowered the number. In the same way, `StackSize 0` (min 1) jumps to
1 on **Left**.

The linear path does not do this. `ff_slider` steps from the clamped `shown` value, and
`snap_value(1000 + 1)` returns 1000, which is unchanged in slider space, so Right is a no-op. Only
Left moves an over-range value, and it moves it downward.

The doc comment of `themed_slider` says "a hand-written out-of-range value must survive; it is
written back only when the user moves the slider". A keypress in the opposite direction breaks
that.

**Fix:** Only step when the key moves the value toward or into the range:
```rust
if delta != 0 {
    let hi = max.max(min);
    let blocked = (delta > 0 && *value >= hi) || (delta < 0 && *value <= min);
    if !blocked {
        let new = ((*value).clamp(min, hi) + delta).clamp(min, hi);
        if new != *value {
            *value = new;
            resp.mark_changed();
        }
    }
}
```
Add a test: start at `v = 1500` with range `0..=1000` and log, focus the slider, press
ArrowRight, and assert that `v == 1500` and the response is not changed.

### WR-02: + Add rule inserts a match-everything `Show` as the first block (accepted, author decision)

**File:** `src/overlay/rules.rs:1097`, `src/filter/structure.rs:205-274`
**Issue:** This is unchanged from iteration 1 and was deliberately skipped (RULES-03). Until the
new rule gets a condition, an Apply makes it shadow every Hide rule, every styled rule and the
economy section. It is recorded here so the risk stays visible. It is not a blocker.

**Fix:** None required by this review. If it is revisited, the lightest option is a non-blocking
row flag icon (no text, in line with "UI not texty") on an enabled, condition-less,
non-Continue rule that has rules below it.

## Info

### IN-01: The number field grows with an unvalidated draft that the slider did not reserve room for

**File:** `src/overlay/widgets.rs:362-363`, `src/overlay/widgets.rs:278`
**Issue:** `number_field` widens to `draft.chars().count()`, but `themed_slider` sizes its reserve
from `[min, max, value]` and never sees the draft. Two cases follow:
- An invalid or very long draft makes the field grow past the row's right edge. Examples are a
  pasted `12345678901234` or text that does not parse, which never commits, so the reserve never
  catches up.
- A valid draft that commits shifts the slider and the field by one frame on each extra digit.

**Fix:** Cap the width, for example at `number_field_width(7)`, and let the TextEdit scroll beyond
that. Alternatively, pass the same width to both widgets.

### IN-02: A scrolling dropdown does not bring the selected option into view

**File:** `src/skin/widgets.rs:850-857`
**Issue:** When the list scrolls (more than 12 options, for example the style picker with many
custom styles), the popup opens at the scroll offset the area last kept, not at the current
choice. The selected row can therefore be off-screen when the popup opens.

**Fix:** On the frame the popup opens, call `ui.scroll_to_rect(row_rect, Some(Align::Center))`
from `dropdown_row` for the row where `i == selected`.

### IN-03: `ff_arrow` returns differently-sized responses and overlapping hit targets

**File:** `src/skin/widgets.rs:1003-1011`
**Issue:**
- An enabled arrow returns the 24x22 hit response, but a disabled arrow returns the 16x16 hover
  response. Callers that read `.rect`, such as tooltip anchoring or the tests, see different
  geometry depending on state.
- The two row arrows are 4pt apart, and each hit target extends 4pt sideways, so the gap
  between them is covered by both. The arrow registered last (Up) wins the whole gap.
- The 16x16 hover widget allocated under an enabled arrow is a second, unlabeled widget.

These are harmless today.

**Fix:** Return a response with `rect` set to the art rect in both states, or document the
difference. Optionally, split the gap by using a 2pt horizontal pad toward the sibling arrow.

### IN-04 (carried IN-01): A title comment directly above the first block moves below the new rule

**File:** `src/filter/structure.rs:280-283`. This is still present (`top_insertion_line` returns
`first.span.start`).

### IN-05 (carried IN-02): Add-then-delete leaves a blank line in two layouts

**File:** `src/filter/structure.rs:248-250`. This is still present.

### IN-06 (carried IN-03): The search box's "Clear" overlays the query text

**File:** `src/overlay/rule_list.rs` (search field). This is still present.

### IN-07 (carried IN-04): `chip_list` `_max_height` is dead; there is no height cap

**File:** `src/overlay/widgets.rs:591`. This is still present. The callers and the new WR-07 test
still pass `160.0`.

### IN-08 (carried IN-05): The `sync_working_styles` doc is attached to `nth_row_id`; the test is not gated

**File:** `src/overlay/app.rs:232-238`, `src/overlay/app.rs:3949`. This is still present:
- The doc block now documents `nth_row_id`.
- `expand_seeds_the_nth_row` calls a `#[cfg(debug_assertions)]` function from a test that is not
  gated, so `cargo test --release` would not compile.

### IN-09 (carried IN-06): Rule rows have no accessible name

**File:** `src/overlay/rule_list.rs:518`. This is still present: `row_resp` gets no `widget_info`.

### IN-10 (carried IN-07): Locked action labels are not greyed; the "Continue to later rules" text is not clickable

**File:** `src/overlay/body.rs` (`card_header`, Continue row). This is still present.

### IN-11 (carried IN-08): The light test skin changes 9-slice geometry in unit tests

**File:** `src/skin/textures.rs:25-49`, `src/skin/mod.rs:60-61`. This is still present.

### IN-12 (carried IN-09): `--expand N` past the last rule is silently ignored

**File:** `src/overlay/app.rs` (screenshot `--expand`). This is still present.

---

_Reviewed: 2026-10-16_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
