---
phase: 49-drag-and-drop-reorder
reviewed: 2026-10-23T00:00:00Z
depth: standard
files_reviewed: 13
files_reviewed_list:
  - src/filter/mod.rs
  - src/filter/structure.rs
  - src/models/config.rs
  - src/overlay/app.rs
  - src/overlay/config_state.rs
  - src/overlay/config_view.rs
  - src/overlay/drag_reorder.rs
  - src/overlay/mod.rs
  - src/overlay/rule_list.rs
  - src/overlay/rules.rs
  - src/overlay/screenshot.rs
  - src/skin/textures.rs
  - src/skin/widgets.rs
findings:
  critical: 0
  warning: 5
  info: 7
  total: 12
status: issues_found
---

# Phase 49: Code Review Report

**Reviewed:** 2026-10-23
**Depth:** standard (phase diff `b563e49..HEAD`, plus the surrounding code it calls)
**Files Reviewed:** 13
**Status:** issues_found

## Summary

I reviewed the drag-and-drop reorder diff: the pure `drag_reorder.rs` maths,
`Document::move_run_to`, `RowAction::MoveTo` / `move_rule_to`, the RULES list and CONFIG style
table drag lifecycles, `StyleSet::move_custom_to`, the handle widget and art, and the debug
screenshot flags.

The document layer holds up. `move_run_to` splices the region (`span.start .. unit.end`) and
moves surplus end-of-file blanks next to the moved run, so the line multiset is preserved. Bytes
of an unterminated last line and CRLF line endings are kept. Destinations at or past the economy
banner, `AfterBanner(economy)` and runs inside the economy section are refused. No-op
destinations return `Ok(false)`, and `commit_structural` rejects any splice that would give lines
to the wrong block. `slot_target` / `is_noop` handle the economy boundary, collapsed headers and
the source's own neighbourhood correctly. `MoveTo` goes through `action_target`, so followers and
economy ids are refused, and `groups.members` moves the whole `# ff:group` run. The screenshot
flags are debug-only: `mod screenshot`, `new_screenshot` and `run_screenshot` are all behind
`#[cfg(debug_assertions)]`. Only the inert `injected` field and its NaN check are compiled into
release builds.

Where it falls short:

- **The drop marker is painted underneath the drag shadow in both lists** (WR-01). In the RULES
  list the marker shows through only because the row grounds are translucent.
- **A drag is not cancelled when reordering becomes disabled mid-drag** (WR-02).
- **The CONFIG "release outside cancels" check uses the whole tab viewport** (WR-03).
- **The `moved across Continue` flag is cleared by a later non-crossing drop** (WR-04).
- **`models` now depends on an `overlay` module** (WR-05).

On the config-drag.png observation: I'm fairly confident this is a real paint-order defect
(classified WARNING, not BLOCKER), but the screenshot also exaggerates it. The `--drag-style`
injection never runs the drag-start collapse, and `--expand-style` kept "mine" expanded, so gap 0
falls on the gold top edge of that expanded editor's frame. A real drag can't reach that state.
The structural problem stands either way: the marker is always inside the shadow and drawn below
it. I couldn't check pixels (no image tooling), so this conclusion comes from the geometry and
the paint order.

## Warnings

### WR-01: The gold drop marker is drawn below the Tooltip-order shadow, and always inside it

**File:** `src/overlay/rule_list.rs:728-738` (marker), `src/overlay/rule_list.rs:748-770`
(shadow); `src/overlay/config_view.rs:1043-1050` (marker), `src/overlay/config_view.rs:1055-1077`
(shadow)

**Issue:** In both lists the marker goes to `ui.painter()`, which is the scroll content layer
(`Order::Middle`). The shadow is an `egui::Area` on `Order::Tooltip`, so it always paints over
the marker. The two always overlap:

- The shadow is centred on the pointer: `shadow_top(py, h, ..)`.
- The slot is the boundary nearest the shadow's centre: `nearest_slot(&offsets, centre, ..)` and
  `style_gap_for_y(&bounds, top + h / 2.0)`.
- Boundaries are at most one row apart (RULES 52pt or a 36pt header; CONFIG 34pt), so the marker
  is never more than h/2 from the shadow's centre.

So the marker is always covered by the shadow, and only shows through the shadow's translucent
parts:

- **RULES** (rules-drag.png): `ROW_EVEN` / `ROW_ODD` / `ROW_HOVER` are low-alpha. The gold line
  shows through, but the shadow's text sits at 60% directly on the text of the rows below it,
  which reads as garbled.
- **CONFIG:** the shadow row's opaque cells (switch art, the Name text field, the preview plate,
  the handle) hide most of the line's width.

The UI-SPEC's only drop affordance is "a gold line between rows". It loses contrast or is mostly
hidden exactly where the user is looking.

**Fix:** paint the marker above the shadow. One way is to paint it on the shadow's own layer after
the row is drawn, with a fresh painter that has full opacity and the list's clip rect:

```rust
// after the shadow Area has been shown (both lists)
if let Some((_, y)) = marker_y {
    let layer = egui::LayerId::new(egui::Order::Tooltip, egui::Id::new(("ff-drag-shadow", "rules")));
    let painter = ctx.layer_painter(layer).with_clip_rect(list_rect);
    let t = drag_reorder::MARKER_THICKNESS;
    let line = egui::Rect::from_min_size(egui::pos2(list_rect.left(), y - t / 2.0), vec2(content_w, t));
    painter.rect_filled(slice::snap_rect(line, ctx.pixels_per_point()), 0.0, pal::GOLD);
}
```

In RULES, `marker_y` is computed inside the scroll closure, so carry it out as a screen-space y.
Also consider giving the shadow an opaque ground (for example a `pal::PANEL` fill before the row
at opacity 0.6) so its text does not mix with the rows under it. Add a test that the last shape
painted in the marker's rect is the gold one.

### WR-02: A RULES drag keeps running after reordering becomes disabled (search typed mid-drag)

**File:** `src/overlay/rule_list.rs:580-616`, `src/overlay/rule_list.rs:625-656`

**Issue:** `reorder_enabled` is checked only when a drag starts (`handle.drag_started() &&
reorder_enabled`). The finish and cancel block at the top of `show_list` never looks at it again.
egui surrenders text focus on a click elsewhere, which fires on release. So a search box focused
before the drag can keep focus while the handle is held, and typing then filters `entries`
mid-drag. The drag continues over the filtered list:

- `slot_entries`, `is_noop` and `slot_target` work on the filtered entries, where neighbours in
  the list are not neighbours in the file.
- Headers are expanded for display while searching.
- Releasing commits a `MoveTo` computed from that view.

This breaks the contract that dragging is off while searching (CONTEXT, UI-SPEC). The resulting
destination is still a real place in the file, so no data is lost, but the drop lands somewhere
other than where the user saw it.

**Fix:** cancel when reordering is no longer allowed:

```rust
if let Some(d) = list.drag {
    if !reorder_enabled && d.injected.is_none() {
        list.cancel_drag(&ctx);
    } else if escape { ... }
```

### WR-03: CONFIG commits a style move when released anywhere in the CONFIG viewport, not just over the table

**File:** `src/overlay/config_view.rs:829`, `src/overlay/config_view.rs:849-860`,
`src/overlay/config_view.rs:1035-1036`

**Issue:** `area = ui.clip_rect()` is the visible viewport of the whole CONFIG scroll area. That
includes the Filter section, the Economy Visibility row and the built-in table. `inside` and
`last_gap` are both tested against `area`, and `style_gap_for_y` always returns a gap: the
nearest one, which is gap 0 for anything above the custom table. So releasing over the Filter
section or the built-in table moves the style to the top of the custom list. The CONTEXT says
"release outside the list cancels", and the RULES list uses its own list rect.

**Fix:** build the drop rect from the custom table itself, for example
`row_rects.first().top() - h/2 .. row_rects.last().bottom() + h/2`, widened to the table's width.
Use it for both `inside` checks. Keep `area` only for the auto-scroll edges.

### WR-04: `moved across Continue` is cleared by a later non-crossing drop even though the rule still sits across the Continue rule

**File:** `src/overlay/rules.rs:1800-1851` (same logic as `move_rule` at 1763-1792)

**Issue:** `move_rule_to` sets the flag only when this one drop passes a Continue rule. Otherwise
it removes the flag. The flag is meant to warn that the rule changed sides of a Continue rule
compared with the applied file, but it is computed per edit, not against the applied order. An
example with A, B (Continue), C, D:

1. Drag A below C. It passes B, so A is flagged.
2. Drag A between C and D. It passes nothing, so the flag is cleared.

A still sits below B, but the warning is gone before Apply. The arrows had the same weakness (two
down steps did the same thing). Drag makes it easier to hit, since every multi-gap drop is one
edit.

**Fix:** decide the flag against the last applied document. Set it when the set of Continue rules
above the moved rule differs from that set in the applied text. Or, at minimum, never clear a
flag set earlier in the session until Apply or Revert.

### WR-05: The model layer now depends on an overlay (UI) module

**File:** `src/models/config.rs:1022`, `src/models/config.rs:1025`

**Issue:** `StyleSet::move_custom_to` calls `crate::overlay::drag_reorder::style_noop` and
`style_gap_to_index`. `models` is used by the CLI binaries (`generate`, `apply`, `sync`), and this
is its first dependency on `overlay`. It builds today because `overlay` is not cfg-gated. But it
inverts the layering: a future cfg-gating of the GUI module would break the CLI build, and the
gap maths now lives in a UI file.

**Fix:** move the two helpers into `models` (or a small shared `crate::reorder` module) and
re-export or call them from `drag_reorder`.

## Info

### IN-01: The arrow move paths are now dead in production

**File:** `src/overlay/rules.rs:116`, `src/overlay/rules.rs:1763`, `src/overlay/rules.rs:263-264`,
`src/overlay/rules.rs:331-332`; `src/overlay/config_view.rs:76`, `src/overlay/app.rs:3171`

**Issue:** No view emits `RowAction::Move` or `ConfigAction::MoveStyle` any more. Their code is
reachable only from tests: `move_rule`, `can_move_up` / `can_move_down` on `RuleRow`, the
`MoveStyle` handler in `app.rs`, `ConfigState::move_style` and `StyleSet::move_custom`.

**Fix:** remove them, or mark them clearly as test or fuzz support if `tests/overlay_rules` needs
them.

### IN-02: The injected RULES pointer uses the one-shot scroll target, not the current scroll offset

**File:** `src/overlay/rule_list.rs:563-572`

**Issue:** `list_rect.top() + y - offset.unwrap_or(0.0)` reads `offset`, which is the pending
`scroll_top` / `pending_scroll` target (usually `None`), not the scroll area's current offset. In a
scrolled screenshot the injected pointer lands on the wrong boundary. The comment says "at its
current scroll offset". Debug-only.

**Fix:** resolve the injected position inside the scroll closure from `viewport.min.y`, as
`draw_style_drag` resolves its gap after layout.

### IN-03: Screenshot injection skips the drag-start collapse, so the drag screenshots show a state a real drag can't reach

**File:** `src/overlay/app.rs:1386-1408`

**Issue:** Real drags set `expanded = None` (and in CONFIG bump the epoch) at drag start. The
injected `DragState` / `StyleDrag` does neither, so `--drag-style` combined with `--expand-style`
renders an open editor during a drag. That is why config-drag.png looks the way it does (see the
Summary).

**Fix:** when injecting, clear `list.expanded` / `config_ui.expanded`. Get the CONFIG table into
view some other way, for example with a scroll flag.

### IN-04: The source-row wash differs between the lists and uses a magic colour

**File:** `src/overlay/config_view.rs:1036-1040` vs `src/overlay/rule_list.rs:1517-1521`

**Issue:** CONFIG uses `Color32::from_rgba_unmultiplied(0x0A, 0x0A, 0x09, 140)`. RULES uses
`pal::GROUND.gamma_multiply(0.55)`, which is what the UI-SPEC prescribes.

**Fix:** use `pal::GROUND.gamma_multiply(0.55)` in both, or define a shared palette constant.

### IN-05: The focus-loss cancel is silently off when focus is unknown, and some resets don't stop egui's drag

**File:** `src/overlay/rule_list.rs:1337`, `src/overlay/config_view.rs:1284`;
`src/overlay/rule_list.rs:182` (`clear_ids`), `src/overlay/config_view.rs:197`
(`reset_for_new_styles`)

**Issue:**

- `had_focus` is `focused.unwrap_or(false)`. When the platform reports no focus state at drag
  start, losing focus never cancels the drag.
- `clear_ids` and `reset_for_new_styles` drop the drag state without `ctx.stop_dragging()`. egui
  keeps dragging the old handle id until release, which suppresses every handle's hover art and
  tooltip until then.

Both are harmless for correctness.

**Fix:** call `stop_dragging` in both reset paths, or route them through `cancel_drag`.

### IN-06: `move_rule_to` / `move_run_to` don't check that a `Before(id)` destination is a group leader

**File:** `src/overlay/rules.rs:1800`, `src/filter/structure.rs:532-578`

**Issue:** `MoveDest::Before(follower)` inserts the run inside another `# ff:group` run and
splits it. Today's UI always sends a leader (`rows[i].id`), so this can't happen from the
overlay. The API itself accepts the destination, though, and the result reloads as `group
broken`.

**Fix:** in `move_rule_to`, map `Before(id)` to `Before(self.groups.rule_of(id))`, or refuse it
when the id is not a leader.

### IN-07: A blank-less neighbour can make a valid drop fail closed (not verified)

**File:** `src/filter/structure.rs:603-635`

**Issue:** The moved region keeps its own leading comments and trailing blanks, but the block it
lands after keeps only its own (possibly empty) trailing blanks. Take a hand-written file whose
previous block runs straight into the destination: no blank before `Before(x)`, before a user
banner, or before the economy banner. After the splice, the moved run's leading `#` comment sits
directly under that block's body. If the parser counts that comment as part of the block above,
`commit_structural` refuses the move (`WouldReattributeLines`) and the user sees a refusal for a
legal drop. It fails closed, so no data is lost. Files the overlay writes always have the blank
(`divider.rs` adds one).

**Fix:** when the line just before `ins` is not blank, insert the moved region's first trailing
blank before it as well (the multiset stays the same). Add a test with a block-into-block file.

### Note: `arrows_w` naming is stale

**File:** `src/overlay/rule_list.rs:1347`

`arrows_w` now measures the handle. Renaming it to `handle_w` would keep `flag_budget`'s input
self-explanatory. This is cosmetic and not counted.

---

_Reviewed: 2026-10-23_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
