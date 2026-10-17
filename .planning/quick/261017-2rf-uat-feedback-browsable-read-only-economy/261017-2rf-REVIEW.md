---
phase: quick-261017-2rf
reviewed: 2026-10-17T00:00:00Z
depth: standard
files_reviewed: 11
files_reviewed_list:
  - src/overlay/rule_list.rs
  - src/overlay/rules.rs
  - src/overlay/app.rs
  - src/overlay/screenshot.rs
  - src/overlay/widgets.rs
  - src/overlay/shell.rs
  - src/overlay/config_view.rs
  - src/overlay/style_lines.rs
  - src/overlay/body.rs
  - src/skin/widgets.rs
  - src/skin/editor.rs
findings:
  critical: 0
  warning: 5
  info: 7
  total: 12
status: issues_found
---

# Quick 261017-2rf: Code Review Report

**Reviewed:** 2026-10-17
**Depth:** standard (scope: `git diff cba5a95..HEAD -- src/`)
**Files Reviewed:** 11
**Status:** issues_found

## Summary

I found no way for an expanded economy row to change the filter or the real `ListUi`. The body
runs against `list.clone()` and a local sink, and `show_list` binds the result to `_discarded`.
egui 0.36.1 turns off clicks, drags, focus, Tab focus and wheel scrolling for disabled widgets
(`create_widget` sets `interested_in_focus` only when the widget is enabled, and `ScrollArea`
checks `ui.is_enabled()`). Every `remember(...)` write in `style_lines` happens only after a click
or change, and every `request_focus` needs a `focus_card`/`focus_line`/`focus_name` naming the
economy block, which nothing can set. `dispatch` still refuses economy targets (T-32-13).
`kept_positions` / `rekey_after_apply` look up only user rows (`working.row`), so they never point
an economy id at a user row. The delete confirm is cleared both in `show_list` and on an economy
click. The economy collapsed-state key stays `ECONOMY_SECTION_KEY` even though the toggled name
is now "FF Economy Section". The `ff_button_ex` `ui.is_enabled()` change only affects buttons
already inside a disabled `Ui`. Today those are the economy body, non-editable user rows and the
`add_enabled_ui(!none)` beam row, and all of them now read as disabled, which is correct.
`cargo test --lib economy` passes (65 tests).

The problems:
- **Tooltips:** egui never shows a hover tooltip on a disabled widget. Truncated base types, cut
  captions and control explanations in the read-only body cannot be read, which works against
  the point of the feature.
- **Minimap hint (IN-04):** the remembered value takes priority over the hint and is kept per
  BlockId across re-parses, so a stale value can win. Turning the Minimap check off and on again
  loses the line's colour and shape.
- **Tests:** two of the new tests prove less than their names claim. One assertion always passes,
  and one test cannot fail.

## Warnings

### WR-01: Hover tooltips never show in the read-only economy body, so truncated text cannot be read

**File:** `src/overlay/rule_list.rs:966` (with `src/overlay/widgets.rs:715`, `src/skin/editor.rs:239`)
**Issue:** `economy_body` calls `ui.disable()`. In egui 0.36.1, `Response::on_hover_text` /
`on_hover_ui` go through `Tooltip::for_enabled`, which opens only when
`response.enabled() && ...`. Every widget in a disabled `Ui` is `enabled == false`, including
hover-only rects such as the `string_list` rows. So in an expanded economy rule:
- A long BaseType entry is cut with "…" to one line (`widgets.rs:707-715`), and its
  `row.on_hover_text(item)` full-name tooltip never appears. Economy blocks are mostly long base
  lists, so this is the main thing a user wants to read.
- The new "full text on hover" for a shortened colour-card caption (`editor.rs:239`) never
  appears either.
- The operator, condition and style explanations, which the project keeps in tooltips
  ("standing explanations are tooltips on the control"), all disappear.

CLAUDE.md says the body is inspectable. Today it hides the very text that is cut.
**Fix:** In the widgets that show a tooltip as a substitute for content, show it whether the
widget is enabled or not. For example, add a helper and use it in `string_list` rows,
`ff_colour_card_ex` and the cut summary / operator labels:
```rust
pub fn hover_text_any(resp: egui::Response, text: impl Into<egui::WidgetText> + Clone) -> egui::Response {
    resp.on_hover_text(text.clone()).on_disabled_hover_text(text)
}
```
Add a browse-test assertion that hovering a cut BaseType row in the economy body opens a tooltip.

### WR-02: The minimap hint ranks below a remembered value that is keyed by BlockId and survives re-parses

**File:** `src/overlay/style_lines.rs:563-568`, `src/overlay/body.rs:3131`
**Issue:** `shown = value().or(recall(memo,"minimap-set")).or(view.minimap_hint)`. The memo id
is `egui::Id::new(("rule-style-last", id))`, with no epoch. It lives in egui temp memory and is
never cleared by `ListUi::clear_ids`. BlockIds restart at 0 on every parse
(`filter/parse.rs:128`), so after an Apply, Revert or Reload that shifts blocks, another rule can
inherit the old memo.

Example: rule A at id 5 had `MinimapIcon 0 Blue Kite` and the user clicked `/` (remembered
Blue Kite). Delete a rule above it, then Apply. Rule B, which reads `MinimapIcon -1 Red Star`, is
now id 5. Its pickers show Blue Kite, and leaving `/` writes `MinimapIcon 0 Blue Kite`, not the
Red Star its own line names.

Putting the hint second gains nothing. The overlay only ever writes a bare `MinimapIcon -1`
(`body.rs:2948/2958`), so a non-`None` hint can only come from the file and is always the more
trustworthy source. This also differs from the plan, which says the hint comes first, then the
remembered value.
**Fix:** Put the hint first:
```rust
let shown = current.value().cloned()
    .or_else(|| view.minimap_hint.clone())
    .or_else(|| recall(ui, memo, "minimap-set"))
    .unwrap_or_else(default_minimap);
```
Separately, salt `rule-style-last` with an epoch that `clear_ids` bumps (for example
`list.row_epoch`, or a dedicated counter), so remembered style values do not leak across
re-parses.

### WR-03: Turning the Minimap check off and on loses the colour and shape of a `MinimapIcon -1 <colour> <shape>` line

**File:** `src/overlay/style_lines.rs:505-524` (called at `548-555`)
**Issue:** Unticking the Minimap check on a rule whose line is `MinimapIcon -1 Red Star`
remembers `Cancelable::Cancel` under `"minimap"`. Nothing is stored under `"minimap-set"`,
because `Cancel.value()` is `None`, and the line is removed. Ticking it again restores `Cancel`,
which writes a bare `MinimapIcon -1` (`body.rs:2958`). `minimap_hint` is then `None`, so leaving
`/` falls back to White Circle. A simple off/on round trip silently drops the tokens the change
set out to keep (IN-04), and the file no longer matches what was loaded.
**Fix:** In `minimap_group`, when unticking a `Cancel` that has a hint, also remember the hint:
```rust
if !now && cancel && let Some(h) = &view.minimap_hint {
    remember(ui, memo, "minimap-set", h);
}
```
If the bytes should come back exactly, remember the original tokens and write
`MinimapIcon -1 <colour> <shape>` on re-tick. Add a test: off, then on, then click the Red
swatch, which should write `MinimapIcon 0 Red Star`.

### WR-04: Assertion "the body was drawn" always passes

**File:** `src/overlay/rule_list.rs:2955`
**Issue:** `assert!(list.expanded_h > ROW_H, "the body was drawn")`. `ListUi::default()` sets
`expanded_h = 180.0` and `ROW_H = 52.0`. `click_at` runs its last frame on the release event,
which is the frame that sets `list.expanded`, so no later frame ever draws or measures the body.
The assertion holds even if `draw_economy_row` never draws a body.
**Fix:** Run one or two more empty frames after the click, then assert against a real
measurement, for example `assert!((list.expanded_h - 180.0).abs() > 0.5 && list.expanded_h > ROW_H + 40.0)`.
Alternatively set `list.expanded_h = 0.0` before clicking.

### WR-05: The "browse then Apply is byte-identical" test cannot fail, and the real `show_list` discard is never clicked through

**File:** `src/overlay/app.rs:4157-4181`, `src/overlay/rule_list.rs:1647`
**Issue:** `economy_browse::browse(&working)` takes `&WorkingFilter` and builds its own `ListUi`,
so it cannot mutate `working`, and the app's `list` is never involved. `report.actions` is
asserted empty, so the `apply_action` loop runs zero times. The test compares two Applies of the
same text, which only shows that Apply is deterministic. Also, `browse` calls
`draw_economy_row` directly. The production guarantee is the `let _discarded = ...` in
`show_list:589-590`, together with `frame.actions` reaching `run_row_actions`, and no test clicks
body widgets through `show_list`/`OverlayApp`. `economy_rule_header_click_collapses...` only
clicks the header. If `show_list` were later changed to `frame.actions.extend(draw_economy_row(...))`,
or the body were given `list` instead of the clone, the browse tests would still pass.
**Fix:** Drive `browse` through `show_list` with a mutable `ListUi` that the caller owns and
return `ListFrame.actions`. In the app test, run real frames of `OverlayApp` (or
`show_list` + `run_row_actions` on `app.list` / `app.working`) with the clicks and keys, then
assert `!working.is_dirty()` and identical bytes. Drop the dead `for a in report.actions` loop.

## Info

### IN-01: Apply closes an open economy row, though Apply keeps an open user row open

**File:** `src/overlay/rule_list.rs:172-181`
**Issue:** `kept_positions` maps `expanded` through `working.row(id)`, which finds user rows only.
An expanded economy rule becomes `None`, and `rekey_after_apply` leaves it closed. The doc
comment says "Apply is not a reload: the open row stays open". This is harmless but inconsistent.
**Fix:** Also record the economy position (`economy_rows().iter().position(..)`) in
`KeptPositions`, and re-key it against the re-parsed `economy_rows()`.

### IN-02: Debug `--dropdown sound` can open a live popup from inside the disabled economy body

**File:** `src/overlay/style_lines.rs:853-856`
**Issue:** `take_dropdown_request` + `Popup::open_id` run even in a disabled `Ui`. With
`--expand-economy` and `--dropdown sound`, the economy body uses up the request and opens the
sound list. The popup is its own `Area` and is enabled, and its picks go to the discarded sink.
This is debug-only, but it breaks the "no popup opens" claim.
**Fix:** Guard it with `ui.is_enabled() && take_dropdown_request(...)`.

### IN-03: Nested disables fade the economy body twice

**File:** `src/overlay/rule_list.rs:965-966`, `src/overlay/style_lines.rs:600-602, 622-624`, `src/overlay/body.rs:3708`, `src/skin/widgets.rs:1155`
**Issue:** Each `ui.disable()` / `add_enabled_ui(false)` multiplies the painter opacity again,
so a disable inside the economy body ends up near 0.36. `ff_button_ex` also switches to the
`DISABLED` caption and a 90-alpha tint. Examples are the minimap shapes and size under `/`, the
beam "Temporary" toggle under None, and `+ condition` / `+ Add`. The rest of the body uses 0.6
for readability, but these parts become almost invisible.
**Fix:** Accept it, or avoid compounding: for example skip `ui.disable()` when
`!ui.is_enabled()`, and/or give `ff_button_ex` a lighter tint when the parent `Ui` is already
disabled.

### IN-04: The read-only body still draws editing controls

**File:** `src/overlay/rule_list.rs:976-985`
**Issue:** `editable: working.body_editable(row.id)` is `true` for economy blocks. The body
therefore draws `+ condition`, `+ style`, per-entry remove X buttons, the "+ Add" field and
button of every quoted list, and the condition remove buttons, all disabled. That adds clutter
in a view that can never edit. CLAUDE.md memory: "Overlay UI not texty / avoid noise".
**Fix:** Pass a `read_only` flag in `BodyCtx` and skip the add/remove controls when it is set.

### IN-05: The browse test's coverage is narrower than its name says

**File:** `src/overlay/rule_list.rs:1613-1631, 1647`
**Issue:**
- `snapshot()` leaves out `advanced_defences`, `pending_scroll`, `scroll_top`, `section_focus`,
  `epoch` and `row_epoch`, yet the assertion reads "the list state is untouched".
- `browse` passes `classes: None`, so the grouped class picker (`body.rs:2162-2165`, which writes
  temp state on click) never runs.
- `rich_economy_text` has no `# ff:name`, so the economy Name row (`rule_list.rs:967-973`) is
  never drawn or clicked.

**Fix:** Snapshot the whole `ListUi` apart from `expanded` / `expanded_h` / `last_expanded`. Pass
a real `ClassCatalog`, and add a named economy block.

### IN-06: A user section named "FF Economy Section" looks the same as the economy header

**File:** `src/overlay/rules.rs:240, 882-886`; `src/overlay/rule_list.rs:450`
**Issue:** The settings key is protected (`section_key`), but nothing stops a user from naming a
section "FF Economy Section". It then has the same caption and the same accessible name as the
locked economy header. Only the padlock tells them apart.
**Fix:** Add `economy` to `section_accessible_name` (for example "…, generated, read-only"), or
refuse or rename that exact name on rename.

### IN-07: Small leftovers

**Files:** `src/overlay/config_view.rs:443-445`, `src/overlay/rule_list.rs:855-857`
**Issue:**
- `TIER_UNIT_GAP` is 10pt, but the plan says 8pt.
- `TIER_FIELD_W` / `TIER_UNIT_GAP` are `pub` but used only inside the module.
- Every economy row has the same AccessKit name, "Economy rule (read-only)", with no index,
  name or expanded state, so a screen reader cannot tell the rows apart.

**Fix:** Align the plan or the constant, make the consts `const` (private), and label economy
rows like user rows, for example `format!("Economy rule {n}, read-only, {expanded|collapsed}")`.

---

_Reviewed: 2026-10-17_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
