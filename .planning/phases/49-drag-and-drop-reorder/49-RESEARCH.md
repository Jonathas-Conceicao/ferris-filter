# Phase 49: Drag-and-Drop Reorder - Research

**Researched:** 2026-10-23
**Domain:** egui 0.36.1 pointer-drag handling in a virtualized list and a plain scroll area, banner-aware document move, style order
**Confidence:** HIGH on egui behaviour (read from the 0.36.1 source), HIGH on the repo map, MEDIUM on the shadow-rendering approach (needs the early spike)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

#### Handle (author)
- Art: `assets/custom/icon_reoder_normal.png`, `icon_reoder_hover.png`, `icon_reoder_pressed.png`
  (92x92). Commit them **under these exact names** with their `.ase` sources
  (`icon_reoder_*.ase`) and the three palette files (`button-normal-palllet.ase`,
  `button-hover-palllet.ase`, `button-prassed-pallet.ase`). The spellings are the author's; do not
  rename.
- Placement: where the arrows were (rule rows: right edge; style table: the Order column). Built-in
  style rows show no handle but keep the column so the table lines up.
- Size: about the row's control height (~28pt), scaled down from 92px.
- The art (like the game button art) has transparent margins to the left and below the plate —
  compensate when positioning (ART-04 from Phase 48).
- Section headers keep their arrows; no drag handle on headers. No keyboard reordering (never
  required; not even a backlog item).

#### Drag behaviour (author)
- Press the handle to start; the source row is greyed in place; an expanded row collapses when its
  drag starts.
- Shadow: the collapsed row at ~60% opacity, following the cursor vertically, aligned to the list
  horizontally.
- Drop marker: a gold line between rows.
- Mouse wheel scrolls during a drag; within ~40pt of the top / bottom edge it auto-scrolls, faster
  closer to the edge (time-based, repaint every frame while dragging).
- Release on a valid slot moves the row as **one pending edit** (Apply writes, Revert undoes);
  release outside the list or Escape cancels (also cancel if the window hides / loses focus).

#### Rules kept from the arrows (author)
- A grouped rule (`# ff:group` run) moves as one unit; a move across a Continue rule is flagged
  `moved across Continue`; dropping past a banner — or on a collapsed section header — puts the rule
  in that section (on a collapsed header: as its first rule); nothing lands in or below the economy
  section and economy rows have no handle; dragging is off while searching; custom styles never land
  above the built-ins.

### Claude's Discretion
- Implementation (research: `Sense::drag()` on the handle only, drag state at list level, drop slot
  from the virtual list's offsets, shadow on a Tooltip-order layer, a new `Document::move_run_to`
  with a banner-aware insertion point and `RowAction::MoveTo`, a pure `drag_reorder.rs` shared by
  both lists). Spike wheel scrolling during a drag early.

### Deferred Ideas (OUT OF SCOPE)
- 999.16 (move to top / bottom, into a named section menu, + Add rule insert position) stays in the
  backlog (author).
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| DND-01 | Drag a handle (author art) to reorder rules and custom styles; arrows gone from rule and style rows, section headers keep theirs | Handle widget with `paint_visible` + `[0,0,84,84]` opaque box (same as `ICON_BUTTON_OPAQUE`); `Skin.drag_handle` via `tex!(ctx, "custom/icon_reoder_*.png")`; `click_and_drag` sense (see Pitfall 3); replace `ff_arrow` use in `draw_row` and the Order cell of `style_row` only |
| DND-02 | Source greyed, translucent copy follows cursor, marker, wheel scrolls, edge auto-scroll | **Critical finding:** egui's `ScrollArea` ignores the wheel while any widget is dragged (`ctx.dragged_id().is_none()` gate). Forward `smooth_scroll_delta` and the auto-scroll through `Ui::scroll_with_delta_animation(.., ScrollAnimation::none())`. Shadow = `Area` on `Order::Tooltip`, `ui.disable()` then `ui.set_opacity(0.6)`; marker = painter line in the content ui |
| DND-03 | Release on valid slot = one pending edit; release outside list / Escape cancels | Own drag state at list level, finished by `ctx.drag_stopped_id()` plus `pointer.primary_released()` and no Escape; egui itself aborts the drag on Escape; one `RowAction::MoveTo` / `ConfigAction::MoveStyleTo` per drop goes through the existing apply path (`dirty`, `Revert`) |
| DND-04 | Group as unit, moved-across-Continue flag, banner / collapsed header targeting, economy boundary, search off, customs never above built-ins | `entries`-boundary to `MoveDest` mapping table below; `Document::move_run_to` over lines (banner-aware); `StyleSet::move_custom_to` permutes only custom index slots; search gating via `reorder_enabled` |
| DND-05 | Art files committed under the author's names | Files exist untracked in `assets/custom/` (92x92 PNG, colormap; `git check-ignore` says not ignored); `git add` all 9 files unrenamed; `.ase` are inputs only, no code reads them |
</phase_requirements>

## Summary

The phase is a pure-egui feature with no new dependencies. egui is **0.36.1** (`Cargo.lock`; the registry also holds 0.36.2 sources, so read-through was done on 0.36.1 which is what builds) [VERIFIED: Cargo.lock, ~/.cargo/registry/src/*/egui-0.36.1]. `indexmap` is 2.14 (has `move_index`) [VERIFIED: Cargo.lock]. No packages are installed by this phase, so the Package Legitimacy Gate does not apply.

The one real risk is scrolling during a drag. In `egui-0.36.1/src/containers/scroll_area.rs` (~line 1215) the wheel is only consumed when `ui.rect_contains_pointer(outer_rect) && ui.ctx().dragged_id().is_none()`. While the handle is being dragged (`dragged_id() == Some(handle)`) the `ScrollArea` therefore **does not** scroll on the wheel. Drag-to-scroll is `DragScroll::OnTouch` by default so the child drag does not conflict with it (`ff_scroll_area*` do not change it). The fix is small and uniform for both lists: `Ui::scroll_with_delta_animation(delta, ScrollAnimation::none())` writes `pass_state.scroll_delta`, which the enclosing `ScrollArea` takes when it ends (scroll_area.rs ~1094) and applies as `offset -= delta`. Feeding it `input.smooth_scroll_delta` (same sign convention as the area's own wheel code) gives wheel scrolling; feeding it `vec2(0, -speed*dt)` gives auto-scroll toward the bottom. No offset bookkeeping, no `State::load/store`, and it works for the virtualized RULES list and the non-virtualized CONFIG `ff_scroll_area` alike. Fallback if the spike fails: `ff_scroll_area_viewport`'s existing `offset: Option<f32>` parameter (`vertical_scroll_offset`) driven from a `scroll_y` the list records from `viewport.min.y`.

The second big finding: egui keeps the drag alive when the dragged widget stops being drawn (`interaction.rs`: `dragged = prev_snapshot.dragged` until a `Released` event or Escape). Auto-scroll will virtualize the source row away, so the drag must be tracked at list level with the **captured egui `Id` of the handle** (the row's id scope includes `list.epoch`, which is bumped when the expanded row collapses at drag start, so the id cannot be recomputed).

**Primary recommendation:** Build `src/overlay/drag_reorder.rs` (pure maths) + `ff_drag_handle` (skin) + a list-level `DragState` in `ListUi` / `ConfigUi`; scroll via `scroll_with_delta_animation`; move via a new line-surgery `Document::move_run_to(run, MoveDest)` + `StyleSet::move_custom_to`; run the five-item spike (below) before writing the real widgets.

## Architectural Responsibility Map

Single-process egui desktop app; tiers here are the repo's layers.

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Slot maths, auto-scroll speed, validity, shadow clamp | Pure module (`drag_reorder.rs`, no egui types) | — | House rule: logic testable without a window (`virtual_list.rs`, `visibility.rs` precedent) |
| Handle widget, art state, cursor, tooltip | Skin (`src/skin/widgets.rs`, `textures.rs`) | — | All `ff_*` widgets live there; art embedded in `Skin` |
| Drag lifecycle (start, track, finish, cancel), shadow, marker, scrolling | View (`rule_list.rs`, `config_view.rs`) | Pure module | Needs `Ui`, pointer, layers |
| Document mutation (multi-gap move, banner-aware insert, ids follow) | Model (`filter/structure.rs`) | `overlay/rules.rs` (`WorkingFilter`) | Views only emit `RowAction`s; `WorkingFilter::apply_action` owns groups, `moved`, row rebuild |
| Style order | Model (`models/config.rs` `StyleSet`) | `config_state.rs`, `app.rs` action loop | Existing `move_custom` / `move_style` / `ConfigAction::MoveStyle` path |
| Pending edit / Revert / dirty indicator | `app.rs` apply loops (unchanged) | — | A move through `apply_action` / `ConfigAction` is automatically one pending edit |

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| egui | 0.36.1 (locked) | `Sense::click_and_drag`, `Area`, `LayerId`, `scroll_with_delta_animation`, `set_opacity` | Already the UI toolkit [VERIFIED: Cargo.lock] |
| eframe | 0.36.x | window / frames | Already in use |
| indexmap | 2.14 (locked) | `StyleSet` storage; `move_index` / `swap_indices` | Already in use [VERIFIED: Cargo.lock] |
| image | existing | PNG decode in `textures::decode_png` | Existing embed path |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| hand-rolled drag | `egui_dnd` / `egui::DragAndDrop` payload API | `egui_dnd` forbidden (no new deps). `egui::DragAndDrop` (payload) adds nothing here: we need list-level geometry anyway and it still sets `dragged_id`. Not used. |

**Installation:** none. **Package Legitimacy Audit:** no external packages are installed; slopcheck not applicable. Packages removed: none. Flagged: none.

## Architecture Patterns

### System Architecture Diagram

```
 press+move on handle (Sense::click_and_drag, hit 32pt)
        |
        v
 egui interaction (begin_pass): dragged_id = handle Id  <- persists even if handle not drawn
        |
        v
 show_list frame N ---------------------------------------------------------------+
   1. ListUi.drag None && resp.drag_started() -> start: DragState{src BlockId,      |
      handle_id: resp.id (CAPTURED), grab offset}; list.expanded = None             |
   2. drag Some: pointer = input.pointer.latest_pos()                               |
        |-- wheel: scroll_with_delta_animation(smooth_scroll_delta, none)           |
        |-- edge:  speed = autoscroll_speed(dist_to_edge, 40, 900); delta=-speed*dt |
        |-- request_repaint()                                                       |
        v                                                                           |
   3. rows drawn (source greyed); AFTER rows, inside content ui:                    |
        slot = nearest_slot(offsets, pointer_y - origin_y, valid)  [rules]          |
        marker line at origin.y+offsets[slot]  (gold, 2pt)                          |
   4. shadow: Area(Order::Tooltip, fixed_pos x=list.left, y=clamp(ptr.y)-h/2)       |
        ui.disable(); ui.set_opacity(0.6); draw_row(throwaway ListUi, sink actions) |
   5. finish check (top of next frame):                                             |
        ctx.dragged_id() != Some(handle_id)  -> drag over                           |
          commit iff primary_released && !Escape && focused && pointer in list_rect |
                      && slot valid && slot != no-op                                |
          -> ONE RowAction::MoveTo{ id, dest }                                      |
          else cancel (ctx.stop_dragging())                                         |
        v
 app.rs apply_row_actions -> WorkingFilter::apply_action(MoveTo)
   -> groups.members(id) -> Document::move_run_to(run, dest)  (lines surgery, ids follow)
   -> moved-across-Continue computed once -> rebuild rows/layout -> dirty (Apply/Revert)
```

### Recommended Project Structure
```
src/overlay/drag_reorder.rs   # NEW pure: slot_for_y, Slot->MoveDest mapping, validity, autoscroll_speed, shadow_y, no-op test
src/overlay/virtual_list.rs   # unchanged (offsets reused); maybe add slot_boundary_y helper
src/overlay/rule_list.rs      # DragState in ListUi, handle in draw_row, marker + shadow in show_list
src/overlay/config_view.rs    # handle in Order cell, row-rect recording, marker + shadow for custom rows
src/overlay/rules.rs          # RowAction::MoveTo, WorkingFilter::move_rule_to
src/filter/structure.rs       # Document::move_run_to + MoveDest
src/models/config.rs          # StyleSet::move_custom_to
src/overlay/config_state.rs   # ConfigState::move_style_to
src/skin/widgets.rs           # ff_drag_handle, DRAG_HANDLE_OPAQUE
src/skin/textures.rs          # Skin.drag_handle: [TextureHandle;3]
src/overlay/screenshot.rs     # optional --drag flag (debug only)
assets/custom/                # the 9 author files, committed unrenamed
```

### Pattern 1: Scroll during a drag (the spike target)
**What:** forward wheel + auto-scroll to the enclosing `ScrollArea` through pass state.
**When:** every frame while `list.drag.is_some()`, called from inside the scroll content closure (the area takes `pass_state.scroll_delta` when it ends).
```rust
// Source: egui-0.36.1 src/ui.rs scroll_with_delta_animation; containers/scroll_area.rs ~1094, ~1215
use egui::style::ScrollAnimation;
let (wheel, dt) = ui.input(|i| (i.smooth_scroll_delta.y, i.stable_dt.min(0.1)));
let auto = drag_reorder::autoscroll_speed(dist_to_edge, 40.0, 900.0); // signed: <0 up, >0 down, pt/s
// Positive scroll_delta moves content DOWN (offset decreases): wheel forwards as is,
// "scroll toward the bottom at s pt/s" is -s*dt.
ui.scroll_with_delta_animation(vec2(0.0, wheel - auto * dt), ScrollAnimation::none());
ui.ctx().request_repaint();
```
Gotchas: (1) `Ui::scroll_with_delta` (non-`_animation`) uses `style.scroll_animation`, which eases and lags per frame; always use `ScrollAnimation::none()`. (2) A nested vertical `ScrollArea` that ends after the call takes the delta first. The expanded rule body has inner scroll areas (quoted lists), so collapse the expanded row at drag start (do it for **any** expanded row, not only the dragged one; judgement call, see Open Questions). (3) The scroll is applied when the area ends, so newly exposed rows are drawn next frame; `request_repaint()` each frame covers that.

### Pattern 2: Drag state at list level with a captured Id
```rust
// in ListUi (cleared by clear_ids, ignored by rekey_after_apply)
pub drag: Option<DragState>,
pub struct DragState { pub src: BlockId, pub handle: egui::Id, pub grab_dy: f32, pub saw_focus: bool }
// start (inside draw_row, the handle response `h`):
if h.drag_started() && list.drag.is_none() && reorder_enabled {
    list.drag = Some(DragState { src: row.id, handle: h.id, grab_dy: 0.0, saw_focus: true });
    list.expanded = None;                    // CONTEXT: collapse when the drag starts
    ui.ctx().request_repaint();
}
// finish (top of show_list):
if let Some(d) = list.drag.as_ref() {
    let ctx = ui.ctx();
    let still = ctx.dragged_id() == Some(d.handle);   // NOT the handle's Response: it may not be drawn
    if !still { /* decide commit vs cancel, then list.drag = None; ctx.stop_dragging(); */ }
}
```
`list.epoch` is part of the row id scope (`("rule", row.id, list.epoch)`) and is bumped when `list.expanded` changes (`show_list`, `last_expanded != expanded`), so `h.id` of the same row differs after the collapse. Store the Id at start; draw the pressed art from `list.drag.src == row.id`, never from `resp.dragged()`.

### Pattern 3: Handle widget (`ff_drag_handle`)
Mirror `ff_arrow` (hit rect via `ui.interact(rect.expand(2.0), auto_id.with("hit"), sense)`, registered after the row so it wins) and `ff_reload_button` (art state 0/1/2, `slice::paint_visible`).
```rust
// Source: src/skin/widgets.rs ff_reload_button / ff_arrow; src/skin/slice.rs paint_visible
pub const DRAG_HANDLE_OPAQUE: [u32; 4] = [0, 0, 84, 84];
pub const DRAG_HANDLE_SIZE: f32 = 28.0;
pub fn ff_drag_handle(ui: &mut Ui, skin: &Skin, enabled: bool, dragging: bool) -> Response {
    let auto_id = ui.next_auto_id();
    let (rect, hover) = ui.allocate_exact_size(Vec2::splat(DRAG_HANDLE_SIZE), Sense::hover());
    let resp = if enabled { ui.interact(rect.expand(2.0), auto_id.with("hit"), Sense::click_and_drag()) } else { hover };
    resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, enabled, "Reorder"));
    let state = if !enabled { 0 } else if dragging || resp.is_pointer_button_down_on() { 2 }
        else if resp.hovered() && ui.ctx().dragged_id().is_none() { 1 } else { 0 };
    let tint = if enabled { Color32::WHITE } else { Color32::from_white_alpha(90) };
    slice::paint_visible(ui.painter(), &skin.drag_handle[state], DRAG_HANDLE_OPAQUE, rect, tint);
    if enabled { ui.ctx().set_cursor_icon(if dragging { CursorIcon::Grabbing } else if resp.hovered() { CursorIcon::Grab } else { return resp }); }
    resp
}
```
(Sketch; the cursor line needs tidying. `box_uv(size, [0,0,84,84])` is what `paint_visible` uses, so the geometry test is the same as `reload_button` tests.) The layout rect equals the visible plate, so use it directly in the right-to-left cluster.

### Pattern 4: Shadow
`egui::Area::new(Id::new(("ff-drag-shadow", tab))).order(egui::Order::Tooltip).fixed_pos(pos2(list_rect.left(), y)).interactable(false).constrain(false).show(ctx, |ui| { ui.set_width(content_w); ui.disable(); ui.set_opacity(0.6); draw_row(...throwaway ListUi + sink...) })`.
- `Order::Tooltip` **allows interaction** (`layers.rs allow_interaction` is true for every order), so the shadow's inner widgets (the row's `Sense::click`) would be hit-tested under the pointer. `ui.disable()` makes `hit_test` strip their sense (`if !w.enabled { sense -= CLICK/DRAG }`); this is the same read-only approach the economy rows use. `interactable(false)` only affects the Area's own sense.
- `Ui::set_opacity` multiplies every shape the ui's painter adds (rects, text, galleys with `PLACEHOLDER`, images) via `Painter::multiply_opacity`, and child uis inherit the painter, so `ff_check`, the chip icon and the handle art all fade together. This is what "~60%" should use, not per-colour `gamma_multiply`. Calling `disable()` first and `set_opacity` after avoids stacking the disabled alpha (disable multiplies opacity by `visuals.disabled_alpha`, `set_opacity` replaces it).
- Throwaway state: `draw_row` takes `&mut ListUi` and `&mut Vec<RowAction>`; pass a fresh `ListUi::default()` with `expanded: None` and a discarded Vec. `ff_*` widgets that dim themselves when `!ui.is_enabled()` (`ff_reload_button`-style) will look extra dim in the shadow; if the spike screenshot shows it, thread a `shadow: bool` through `draw_row` instead of disabling.
- Marker: paint in the content ui after the rows (`ui.painter().rect_filled(Rect(x range, y-1..y+1), 0, pal::GOLD)` snapped with `slice::snap_rect`), so it is clipped to the viewport and scrolls with the rows.

### Pattern 5: Entry-boundary to `MoveDest` (RULES)
Boundaries are `0..=entries.len()`; boundary `b` lies between `entries[b-1]` and `entries[b]`; its y is `offsets[b]`. First matching row wins:

| prev (`entries[b-1]`) | next (`entries[b]`) | Dest | Valid |
|---|---|---|---|
| economy Header or `EconomyRule` | anything | — | no (in / below economy) |
| user `Header` (collapsed or expanded) | anything | `AfterBanner(ordinal)` | yes (collapsed header: first rule of that section, per CONTEXT) |
| any | `Rule(i)` | `Before(first block of rule i's group)` | yes |
| any / none | user `Header` | `BeforeBanner(ordinal)` (end of the section above; for `b==0` the preamble above the first banner) | yes |
| any / none | economy `Header` or end | `End` (just above the economy banner, else end of file) | yes |

No-op (emit nothing): the dest resolves to where the run already is. In entry space that is `b == src_pos` and `b == src_pos + 1`, plus `AfterBanner`/`BeforeBanner`/`End` that equal the current position (e.g. source is the last rule of its section and the slot is the boundary above the next header). Do the no-op test in the model as well (`move_run_to` returns "unchanged" and `apply_action` reports `changed: false`) so a stale drop is harmless.

Note a collapsed section hides its rules, so the boundary after a collapsed header maps to `AfterBanner`, never to "end of that section". The boundary above the *next* header is the same geometric place and is claimed by the collapsed header (rule 2 beats rule 4).

### Anti-Patterns to Avoid
- **Reading the handle `Response` after the drag starts.** The row can be virtualized out or re-id'd. Use `ctx.dragged_id()` / `ctx.drag_stopped_id()` with the captured Id.
- **Relying on the `ScrollArea` for wheel during the drag.** It is gated off by `dragged_id()`.
- **Easing scroll (`scroll_with_delta`)** for auto-scroll: lags and ignores `dt`.
- **Using `move_rule` repeatedly.** It toggles the Continue flag per step and hops banners by moving the *banner* (`RuleStep::CrossBanner`), which would move sections, not the rule.
- **Recomputing slots from stale offsets.** Compute the slot inside the content closure in the same frame as the rows (offsets are built every frame from `list.expanded_h`).
- **A drag-only handle (`Sense::drag()`)**: a plain tap would fall through to the row's click and toggle expansion (see Pitfall 3).

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Cancel on Escape | own key tracking | egui does it: `interaction.rs` clears `dragged` and `potential_drag_id` when `Key::Escape` is pressed in begin_pass | Our frame just sees `drag_stopped_id() == handle` with no pointer release and Escape pressed. Only `consume_key(Escape)` that frame so nothing else reacts |
| Drag threshold | own press-origin distance | `Sense::click_and_drag()`: `drag_started` only after `max_click_dist` (6pt default) / long hold / leaving the widget | No collapse-on-tap |
| Art margin compensation | custom UV maths | `slice::paint_visible` + `box_uv` (Phase 48) | Same 84/92 gutter as `fouriconbutton*` |
| Scrolling | `State::load/store`, offset bookkeeping | `Ui::scroll_with_delta_animation(.., none())` | Handled by the enclosing area, both lists |
| Opacity | per-colour multiply | `Ui::set_opacity` | Covers text, images, strokes |
| Banner/line surgery | ad-hoc string edits | `commit_structural(new_lines, ids, expected)` like `swap_runs` | Validates that every block's span text is unchanged and ids are attributed correctly |
| Group membership | own run detection | `self.groups.members(id)` / `rule_of` | Existing, already used by `move_rule` |

**Key insight:** every hard part already has a seam; the new code is geometry (pure), one document operation, and wiring.

## Document / Model Map

### `Document::move_run_to(run: &[BlockId], dest: MoveDest) -> Result<(), EditError>` (`src/filter/structure.rs`)
- `MoveDest { Before(BlockId), AfterBanner(usize), BeforeBanner(usize), End }`. Resolve to an insertion line `ins` in the **old** line vector:
  - `Before(id)`: `infos[pos].span.start` (span = leading comments + owned lines, so the rule's comments stay attached).
  - `AfterBanner(k)`: `banners[k].start + 3`, then skip blank lines (exactly `hop`'s `be` in `divider.rs`). `k` is a user banner (refuse `Economy`: `EditError::SectionLocked`).
  - `BeforeBanner(k)`: `banners[k].start`.
  - `End`: the economy banner start if present (`section::find`), else `lines.len()`. Refuse any `ins` inside or after the economy section (`section::find(self)` span), mirroring `in_economy` / `top_insertion_line`.
- Region moved: `infos[first].span.start .. infos[last].unit.end` (unit = span + trailing blank lines), the same region `swap_runs` uses. `run` must be consecutive blocks in file order (a group). If `ins` is inside the region or equal to its start/end after blank-absorption: no-op (return a distinct no-change result, or `Err(NoNeighbour)`-style that `apply_action` maps to `changed:false`).
- Rebuild: `new = old[..]` with the region removed and spliced at `ins` (adjust `ins` by region length when it lies after the region). Blank-line hygiene identical to `duplicate_block` / `swap_runs`: if the moved unit had no trailing blank (it was the file's last unit) add one `blank_line`, and if the unit left behind is now last, keep the file's final-line terminator rules (`was_unterminated` handling in `swap_runs`).
- ids/expected: `ids`/`expected` come from `infos`, remove the run's entries and re-insert them at the position given by the number of blocks whose `header < ins'`; then `commit_structural`. It validates span texts, so a botched splice fails closed (`WouldReattributeLines`) instead of corrupting the file.
- Tests (house style, `structure.rs` tests): move down/up several gaps; into a banner's section top; to the end of a section; above first banner; to `End` with and without economy; group of 2-3 blocks; trailing-newline-less file; CRLF file; no-op positions; refusing an in-economy destination; `serialize` round-trip and "every non-moved line is byte-identical and the multiset of lines is preserved".

### `WorkingFilter::apply_action(RowAction::MoveTo { id, dest })` (`src/overlay/rules.rs`)
- `dispatch()` already refuses followers of a group and economy ids before the `match`; add `RowAction::MoveTo { id, .. }` to `action_target` (returns `Some(id)`), the `block_action` "handled before" arm, and the `Move`-style handling in `coalesce_edits` (no coalescing).
- `move_rule_to`: `a = self.groups.members(id)`; compute `crossing` once = any block of `a` has `has_continue` **or** any rule unit the move passes over has `has_continue` (rows between the old and new position in `layout`; banners do not count). Mirror arrow behaviour: on success `moved.insert(id)` if crossing else `moved.remove(&id)`. This is not identical to cumulative-across-multiple-steps arrow behaviour (an arrow step without Continue clears the flag); calling that out in the plan avoids a "regression" report. Return `done_or(r, Some(id))` so `focus = id` and `app.rs` sets `pending_scroll`.
- `RowAction::Move`, `can_move_up/down` and `move_rule` stay in the model: `tests/overlay_rules.rs:71` (fuzz) and ~25 unit tests in `rules.rs` use them. Only the *UI* stops emitting them. Dead-code is not flagged (pub items in a lib); cleanup belongs to the Release Review phase, not here.
- `app.rs` `apply_row_actions`: add `RowAction::MoveTo` beside `is_move` (line ~884: `let is_move = matches!(action, RowAction::Move(..))` becomes `Move(..) | MoveTo{..}`) so `list.pending_scroll = Some(focus)`. If the destination is a collapsed section, the rule is hidden: `pos_of` returns `None`, so `pending_scroll` is a harmless no-op (see Open Questions on auto-expanding).

### Styles
- `ConfigAction::MoveStyleTo { name: String, to: usize }` (`to` = gap index 0..=n among custom styles) next to `MoveStyle`; add it to the `matches!` list at `app.rs:3046` (dirty/`config_message` path) and handle beside `MoveStyle` at `app.rs:3147` (`state.move_style_to`, clear message, `apply_state` Applied to Unapplied).
- `StyleSet::move_custom_to(name, gap) -> bool` (`models/config.rs`, beside `move_custom`): collect the indices of custom entries (`!is_builtin`), take the custom-name order, remove `name`, insert at `gap` (clamped), then write the names back into the **same index slots** (via `swap_indices` or rebuild). Built-in positions never change, "never above built-ins" holds by construction, and the result equals the old repeated `move_custom` swaps. Returns false when built-in/unknown/no change.
- `ConfigState::move_style_to` wraps it (`config_state.rs:636` pattern). Revert already restores `styles` from `styles_applied`, so Revert undoes it for free; the dirty count compares styles including order (`style order` is mentioned as an edit in `is_dirty` docs).

## Style table (CONFIG tab) specifics

- `show_styles` is inside `w::ff_scroll_area(ui, skin, "config", ...)` (app.rs:3336): a plain, non-virtualized `ScrollArea::show`. Every custom row is drawn every frame, so row rects can simply be recorded: `cfg.row_rects: Vec<(String, Rect)>` (cleared each frame, filled by `style_row`). Boundary ys: `custom[i].rect.top()` for `i < n`, `custom[n-1].rect.bottom()` for gap `n`; with an expanded editor under row `i` use `custom[i+1].top()`.
- Slot = nearest boundary to the shadow centre (clamped); all gaps among customs are valid; nothing else is a slot, so a custom can never land above a built-in and no marker is drawn over the built-in block.
- Collapse `cfg.expanded` at drag start too (keeps slot geometry simple and avoids inner scroll areas stealing the scroll delta).
- Order cell: `ORDER = 56` column in `table_columns`; the handle (28pt) right-aligned in `cols.order.rect(rect)` via the existing `cell_at(.., Layout::right_to_left(..))`. Built-in rows: allocate an empty 28pt cell (no art), keep `probe("row-order", ..)`. `click_rect` ends at `cols.order.x - 2.0`, so the row click does not overlap the handle. Header text `Order` stays right-aligned. Consider narrowing `ORDER` (two arrows 36pt incl. gap are gone) only if the screenshot shows slack; not required.
- `ORDER_TIP` ("Built-in styles keep their order") becomes unused for the empty cell; either drop it or show it as a hover tip on the blank cell (judgement; the UI-SPEC says the cell is blank).
- Style row's `probe("row-up"/"row-down")` test probes are replaced by `probe("row-handle")` for custom rows.
- The shadow for a style row: `style_row` draws into a `cell_at`-based layout, not a free-standing row function; add an `Area` that calls `style_row` with a throwaway `ConfigUi` and sink (same pattern as rules) or paint a simplified row (background, name, preview, icon). Prefer calling `style_row` with `reorder: Some(..)`, `ui.disable()`, discarded actions; spike it.

## Runtime / structural checks that matter

| Check | Result |
|-------|--------|
| Does `ScrollArea` still consume wheel while a child is dragged? | **No.** Gate `ui.ctx().dragged_id().is_none()` at scroll_area.rs ~1215 [VERIFIED: egui-0.36.1 source] |
| Does a child drag inhibit drag-to-scroll? | Moot: `ScrollSource::default()` has `drag: DragScroll::OnTouch`, so with a mouse there is no background drag response; a touch screen would add one, but `Sense::drag` on the handle registers later and wins its hit area [VERIFIED: scroll_area.rs 192-216, 831] |
| Programmatic scroll | `ScrollArea::vertical_scroll_offset(f32)` (already exposed as `offset` of `ff_scroll_area_viewport`), `Ui::scroll_with_delta_animation` (recommended), `State::load/store(ctx, id)` (needs the area id; `ff_scroll_area_viewport` hides `out.id`) [VERIFIED: ui.rs 1490, scroll_area.rs 75-81, 508] |
| Drag outlives the widget? | Yes: `interaction.rs` keeps `dragged = prev_snapshot.dragged` until `Released` or Escape; comment says the widget "is now in the air" [VERIFIED] |
| Escape | begin_pass clears `dragged`/`potential_drag_id`; `Context::drag_stopped_id()` is set that pass; `pointer.primary_released()` is false |
| Release detection | `ctx.drag_stopped_id() == Some(handle)` and `input.pointer.primary_released()`; pointer position `input.pointer.latest_pos()` (None if the pointer left the window: treat as cancel) |
| Hover during drag | egui reports no widget hovered while one is dragged; `row_background` uses `ui.rect_contains_pointer(rect)` (not the hover state), so rows under the pointer still tint; suppress for the source row (draw its stripe without hover) |
| Time | `ui.input(|i| i.stable_dt)`, cap at 0.1 as `ScrollArea` does |
| Focus | `ctx.input(|i| i.viewport().focused)` is `Option<bool>` (what `app.rs:3675` reads); `InputState.focused: bool` also exists |
| Hide on focus loss | `visibility::advance` conceals on focus loss (ShownFocused to Hidden). Call `list.drag = None; cfg.drag = None; ctx.stop_dragging()` when the Conceal effect fires (app.rs ~3700), and defensively cancel when a drag that had focus loses it. Do not cancel on `focused == Some(false)` alone: a never-activated window reads unfocused until the first click (the `ShownUnfocused` state exists for exactly that) |
| Repaint | Call `ctx.request_repaint()` every frame while any drag is active; the app also has `request_repaint_after(50ms)` heartbeat, so it works without extra plumbing |

## Existing tests to rewrite or keep

Rewrite (arrows on rule / style rows go away):
- `rule_list.rs::arrows_do_nothing_when_reorder_is_disabled_and_work_when_enabled` (~2503): becomes "handle disabled while searching emits nothing; a drag from the handle to another row emits `MoveTo`". Its click sweep (x 380..533) over the old arrow area needs the handle's x range.
- `config_view.rs::custom_arrows_reorder_and_builtin_arrows_are_disabled` (~2557) and the Order-cell arrow geometry test (~2226, probes `row-up`/`row-down`): rewritten for `row-handle` (custom rows have one, built-in rows none, same right edge) and `moves()` helper (~2552) now matches `MoveStyleTo`.
- `rule_list.rs::flag_budget_keeps_label_minimum` (~2096): uses `36.0` as `arrows_w`; update to the handle width + 4.
- Tests that sweep clicks across rows and assert actions (`browse_list`, `browse`, `pending_scroll_brings_a_far_row_into_view`, `painted_row_heights_match_the_virtual_list`) need rechecking since the row's right cluster changes; `app.rs:6049` literal `can_move_up: false` stays (fields stay).

Keep unchanged: section-header arrow tests (`user_header_cluster_emits_remove_move_and_rename`, `MoveSection` tests in `rules.rs`/`divider.rs`), `ff_arrow` unit tests in `widgets.rs` (still used by section headers), all `Move`/`can_move_*` model tests and `tests/overlay_rules.rs` fuzz, `models/config.rs::move_custom_skips_builtins_and_stops_at_ends`, `config_state.rs` `move_style` tests, `app.rs:4961` rig test.

New tests:
- `drag_reorder.rs` pure: nearest slot over `offsets` (fixed, with expanded row), validity table above, no-op detection, `autoscroll_speed` (0 at/after 40pt, linear, 900 at 0, sign by edge, beyond the edge clamps to max), shadow y clamp.
- `structure.rs`: `move_run_to` list above.
- `rules.rs`: `MoveTo` (group as unit, Continue flag from crossed rules, banner top/end, collapsed header target, refuses economy dests, followers refused, `changed:false` on no-op).
- `config.rs`/`config_state.rs`: `move_custom_to` equals repeated `move_custom`; built-ins fixed; Revert restores.
- `widgets.rs`: handle plate rect 28pt, UV equals `box_uv(size,[0,0,84,84])`, disabled senses hover only, tap on handle does not click the row.
- `rule_list.rs`: headless drag tests (press, move, wheel events, release) for start/collapse, wheel scroll, auto-scroll, virtualized-out source, release outside, Escape, shadow rect left == list left, marker y at a valid slot, no marker on invalid.
- `textures.rs`: the three handle textures load (`load_light` covers the embed path).

## Common Pitfalls

### Pitfall 1: Wheel does nothing during the drag
**What goes wrong:** `ScrollArea` skips the wheel while `ctx.dragged_id()` is `Some`.
**Why:** scroll_area.rs ~1215 `ui.ctx().dragged_id().is_none()`.
**How to avoid:** forward `smooth_scroll_delta` yourself (Pattern 1). A headless test must fail without the forward and pass with it.
**Warning signs:** auto-scroll works but the wheel does not (they are separate code paths in our design).

### Pitfall 2: Row id changes at drag start
**What goes wrong:** collapsing the expanded row bumps `list.epoch`, the row's id scope `("rule", row.id, epoch)` changes, so the handle's `Id` differs; checks like `list.drag.handle == h.id` fail and the drag appears to end at once.
**How to avoid:** capture `resp.id` at `drag_started`; never recompute; draw pressed art from `drag.src`.

### Pitfall 3: Tap on a drag-only handle toggles the row
**What goes wrong:** hit-testing returns the row as `hits.click` when the handle only senses drag (`hit_test_on_close`, the "two normal small widgets" arms), so press+release without moving expands/collapses the row.
**How to avoid:** `Sense::click_and_drag()` on the handle so it owns both hits (it is also what delivers the 6pt threshold, matching the CONTEXT "press the handle to start"); ignore `clicked()`. Test: tap emits nothing and leaves `expanded` unchanged. Side effect: the handle is Tab-focusable like `ff_arrow` already is; accessible name `Reorder`, no key handler (CONTEXT: no keyboard reordering).

### Pitfall 4: Source virtualized out mid-drag
**What goes wrong:** auto-scroll moves the source row out of `visible_range`; code that finishes the drag from the handle's `Response` never runs.
**How to avoid:** list-level state and `ctx.dragged_id()`/`drag_stopped_id()` (Pattern 2); the source row's grey wash only needs to paint when it is in range.

### Pitfall 5: Banner semantics differ from arrows
**What goes wrong:** reusing `rule_step` makes a rule that "crosses" a banner move the banner (`CrossBanner`), not the rule.
**How to avoid:** `move_run_to` with explicit `MoveDest`; the mapping table above; tests per row of the table.

### Pitfall 6: Collapsed section ambiguity
**What goes wrong:** the boundary between a collapsed header and the next header is both "end of section above (hidden rules)" and "start of the collapsed section".
**How to avoid:** the table's rule order (prev is a user header wins). Document it in the plan; add a test.

### Pitfall 7: Stale ids on commit
**What goes wrong:** the document changes between drag start and release (a Sync/Reload replaces the model).
**How to avoid:** `dispatch` already refuses unknown ids (`row(id).is_none()` check) and followers; make `move_run_to` fail closed on unknown ids. Also cancel the drag on `Reload`/`Revert`/Apply (the paths that call `clear_ids` clear `drag` too).

### Pitfall 8: Inner scroll areas steal the scroll delta
**What goes wrong:** with an expanded rule body (quoted lists use an inner vertical `ScrollArea`), the inner area's end takes `pass_state.scroll_delta` before the outer one.
**How to avoid:** collapse any expanded row at drag start (decision flagged below); in the CONFIG tab collapse `cfg.expanded` as well.

### Pitfall 9: Shadow steals hits or double-draws
**What goes wrong:** widgets inside a Tooltip-order `Area` are hit-testable; duplicate widget ids warn in debug.
**How to avoid:** `ui.disable()` inside the shadow; a distinct `Area` id; `draw_row` inside the Area derives ids from the Area id, so no clash with the list.

### Pitfall 10: Escape also does other things
**What goes wrong:** the frame the drag aborts, `key_pressed(Escape)` is still true for other widgets (section rename, text fields, popups).
**How to avoid:** on the cancel frame call `ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape))` before drawing rows (no row-collapse or search-clear handler on Escape exists today: `body.rs:251`, `rule_list.rs:840/1508` all require their own focus/edit mode, so the risk is small).

### Pitfall 11: Release over the list edge
**What goes wrong:** release exactly on the viewport edge or over the scrollbar strip (`SCROLLBAR_WIDTH + 4` right of content) counts as outside.
**How to avoid:** define "inside" as `list_rect` (the `available_rect_before_wrap` before `ff_scroll_area_viewport`, whole width incl. scrollbar strip); show the marker only when the pointer is inside and the slot valid, so marker presence = drop will commit. For auto-scroll the pointer being *beyond* the edge but over the window counts as max speed (clamped distance); outside the window cancel on release.

## Code Examples

### Pure helpers (`drag_reorder.rs`)
```rust
/// Nearest boundary of `offsets` to content-space `y`, among boundaries `valid` accepts.
pub fn nearest_slot(offsets: &[f32], y: f32, valid: impl Fn(usize) -> bool) -> Option<usize> {
    offsets.iter().enumerate()
        .filter(|(b, _)| valid(*b))
        .min_by(|(_, a), (_, c)| (y - **a).abs().total_cmp(&(y - **c).abs()))
        .map(|(b, _)| b)
}
/// Signed scroll speed in pt/s: negative = up, positive = down. 0 when farther than `zone`
/// from both edges; linear to `max` at the edge; clamped beyond it.
pub fn autoscroll_speed(y: f32, top: f32, bottom: f32, zone: f32, max: f32) -> f32 {
    let up = ((top + zone - y) / zone).clamp(0.0, 1.0);
    let down = ((y - (bottom - zone)) / zone).clamp(0.0, 1.0);
    (down - up) * max
}
pub fn shadow_top(pointer_y: f32, h: f32, top: f32, bottom: f32) -> f32 {
    (pointer_y - h / 2.0).clamp(top, (bottom - h).max(top))
}
```
(A short list, where the viewport is shorter than twice the zone, makes the bands overlap; define the tie as the nearer edge.)

### Texture embed (precedent `custom/titlebar.png`)
```rust
// textures.rs: field `pub drag_handle: [TextureHandle; 3]` and in Skin::load
drag_handle: [
    tex!(ctx, "custom/icon_reoder_normal.png"),
    tex!(ctx, "custom/icon_reoder_hover.png"),
    tex!(ctx, "custom/icon_reoder_pressed.png"),
],
```
`load_light` (tests) already swaps in 2x2 stand-ins for every `tex!`.

### Rule-row cluster change (`draw_row`)
Replace the two `ff_arrow` calls (rule_list.rs 1132-1148) by `let h = ff_drag_handle(&mut right, skin, reorder_enabled, dragging_this)`; `arrows_w = right.min_rect().width()` already measures what was drawn, so `flag_budget` just works (handle 28 + the existing `add_space(4.0)`). Economy rows (`draw_economy_row`) draw no handle: nothing to change, but check that their right padding is not tied to the arrow width.

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `Window::drag` / `egui_dnd` | `Sense::click_and_drag` + own list-level state | n/a | no deps; full control of virtualization and banners |
| `Ui::scroll_with_delta` (animated) | `scroll_with_delta_animation(.., ScrollAnimation::none())` | egui 0.31+ | per-frame control |
| `ScrollArea::drag_to_scroll(bool)` | `ScrollArea::scroll_source(ScrollSource)` with `DragScroll::OnTouch` default | egui 0.32+ | mouse never drag-scrolls by default |

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `Ui::set_opacity` is inherited by child uis and affects galleys / images painted by `painter.text`, `painter.galley(.., PLACEHOLDER)`, `painter.image` as used in `draw_row` | Pattern 4 | Shadow looks fully opaque; fall back to per-colour `gamma_multiply` via a `shadow` flag. Spike S4 settles it. |
| A2 | `ui.disable()` in an `Area` does not tint the custom-painted row in a way that is visually wrong at 60% | Pattern 4 | Add the `shadow: bool` parameter to `draw_row` instead |
| A3 | A nested `ScrollArea` ending after the call steals `pass_state.scroll_delta` (read from source, not run) | Pitfall 8 | If it does not, collapse-all is only a nicety |
| A4 | In a headless test `RawInput` events `MouseWheel` reach `smooth_scroll_delta` the same frame (smoothing may spread it) | Spike | Test must run several frames |
| A5 | Tap on a drag-only handle falls through to the row click (read from `hit_test_on_close`, arms not fully traced) | Pitfall 3 | If wrong, `Sense::drag()` would be enough; `click_and_drag` remains safe |
| A6 | Auto-collapsing *any* expanded row on drag start (not only the dragged row) is acceptable to the author | Open Questions | Author may expect other rows to stay open; low impact |
| A7 | Mirroring the arrows' "set if crossing else remove" for the `moved` flag is the wanted semantics for a multi-gap move | WorkingFilter section | Author may want "flag if net order crosses any Continue rule from the original position"; trivial to change |

## Open Questions (RESOLVED)

1. **Does a drop into a collapsed section reveal the rule?**
   - Known: the rule becomes the section's first rule and is hidden (section stays collapsed).
   - Unclear: whether the author wants the section to expand (or a status message). CONTEXT says no new message; arrows' hop did not expand.
   - Recommendation: do nothing extra (flag in the plan's judgement calls); the section header's `{n} rules` count updates, which is the feedback.
   - RESOLVED: no expand on drop (plan 04).
2. **Collapse every expanded row at drag start, or only the dragged one?**
   - Recommendation: every expanded row (robust slot maths, avoids Pitfall 8).
   - RESOLVED: every expanded row collapses at drag start (plan 04).
3. **Shadow when the source row is taller (expanded) at the moment of the press?**
   - Recommendation: it is collapsed at the start (we set `expanded = None`), so the shadow always uses `ROW_H`.
   - RESOLVED: shadow uses `ROW_H` (plan 04).
4. **`ORDER_TIP` / blank Order cell on built-in style rows**: keep a hover tip or leave blank? UI-SPEC says blank, no dead control.
   - RESOLVED: blank cell, no tooltip (plan 05).

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust nightly toolchain | build / tests | assumed present (`rust-toolchain.toml`) | floating nightly | — |
| egui 0.36.1 source in cargo registry | API verification | yes | 0.36.1 and 0.36.2 present | — |
| Author art files | DND-05 | yes (untracked in `assets/custom/`: 3 PNG + 3 `.ase`, plus the three palette `.ase` listed in git status) | 92x92 PNG | — |
| Game install | none (no ff-art run needed) | n/a | — | — |

No blocking dependency. Debug screenshot mode runs on a desktop session only.

## Spike Plan (do first; each is a headless test or a tiny local change, no UI polish)

Order matters; S1 and S2 can change the architecture, so they are the first plan's first tasks.

- **S1 Wheel + auto-scroll during a drag** (`rule_list.rs` tests, `headless_ctx` + `run_frame` pattern, 60 rules, `RawInput` with `time`/`predicted_dt` advancing): press on a stub handle widget registered with `Sense::click_and_drag`, move 10pt, send `Event::MouseWheel`. Assert (a) with no forwarding the list offset (`viewport.min.y` seen inside the closure) does **not** change (documents the gate), (b) with `ui.scroll_with_delta_animation(smooth_scroll_delta, none)` it does, (c) with the pointer 5pt from the bottom for N frames the offset grows by about `speed*dt*N`. If (b)/(c) fail, switch to the `offset` parameter fallback (record `viewport.min.y` into `ListUi.scroll_y`, pass `Some(clamped(scroll_y + delta))`).
- **S2 Drag survives virtualization and id changes**: start the drag on row 3, auto-scroll until row 3 is outside `visible_range`, bump `list.epoch` (collapse), keep the pointer down, release. Assert `ctx.dragged_id()` stays `Some(captured)` until the release frame and `drag_stopped_id()` equals it exactly once.
- **S3 Tap vs drag + Escape**: tap (press/release, no move) on the handle: no `drag_started`, row not toggled (`click_and_drag`); with `Sense::drag()` for comparison. Escape mid-drag: `dragged_id()` None, `primary_released()` false, `consume_key(Escape)` leaves `key_pressed(Escape)` false for later widgets.
- **S4 Shadow** (debug screenshot): `Area` on `Order::Tooltip`, `disable()` + `set_opacity(0.6)`, `draw_row` with throwaway state; check opacity, no duplicate-id warnings, no hit stealing (a click on the shadow region falls through), first-frame flash. Add `--drag <row>:<slot>` here already (see below) so every later visual check reuses it.
- **S5 Config tab**: same S1 on `ff_scroll_area` (non-virtual) with `style_row` rows; confirm `scroll_with_delta_animation` is taken by the `config` area when called from `show_styles`, and recorded row rects stay valid across a scrolled frame.

Gate: if S1 (b) fails and the fallback also fails, stop and escalate; otherwise proceed. Everything else in the phase is deterministic.

## Debug screenshot support (optional, recommended)

`--screenshot` is a one-shot render (waits for dock + a few frames). Add a debug-only `--drag <row>:<slot>` (rules tab; 1-based row, boundary index) and for the CONFIG tab `--drag-style <name>:<gap>`, parsed in `screenshot.rs` (`ShotArgs.drag: Option<(usize, usize)>`, parse tests like the others, error on malformed). In `app.rs` after start-up (like `shot.expand`): set `list.drag = Some(DragState{ src, handle: Id::NULL-ish, .. })` plus an injected pointer position instead of reading input, and make the lifecycle code take the pointer from a `fn pointer_for(list) -> Option<Pos2>` that returns the injected position when `list.drag.injected` is set (so no egui drag is needed and the finish logic must skip the `dragged_id()` check for injected drags). The pure `drag_reorder` functions then paint source wash, shadow and marker from the injected slot. Compiled out of release (`#[cfg(debug_assertions)]`, as `shot_step`). Add it to the screenshot flag list in CLAUDE.md. Also use `--tab rules` (handles at the right edge, economy rows without) and `--tab config --expand-style X` to check the Order column alignment.

## Validation Architecture

> `.planning/config.json` has `workflow.nyquist_validation: false`, so no formal validation architecture is required. Test conventions for the plan:

| Property | Value |
|----------|-------|
| Framework | built-in `cargo test` (egui headless contexts, `load_light` skin) |
| Quick run | `cargo test -p ferrisfilter drag_reorder` and `cargo test move_run_to` |
| Full suite | `cargo test --locked` (as `ci.yml`) |
| Isolation | any test that spawns a process pins `FERRISFILTER_HOME` / `FERRISFILTER_GAME_DIR` (enforced by `tests/isolation_guard.rs`); the new tests are in-process and resolve no folders |

Wave 0: none needed (no new framework, fixtures live in the test modules).

## Security Domain

`security_enforcement` is not set to false, so briefly:

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V5 Input Validation | yes (internal) | Drop actions carry `BlockId`/ordinals/indices from the UI; `dispatch` refuses unknown ids, group followers and economy blocks; `move_run_to` and `move_custom_to` fail closed (`commit_structural` re-validates every block span; `gap` is clamped) |
| V2/V3/V4/V6 | no | no auth, sessions, access control or crypto touched |

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Stale ids after a reload mid-drag | Tampering (of the wrong rule) | cancel drag on `clear_ids`; refuse unknown ids |
| Lines lost / duplicated by a bad splice | Tampering (user file corruption) | `commit_structural` span-text check, `.ffbak` first-write backup (existing), round-trip tests |
| Writing outside the game folder | n/a | no new I/O: edits stay in memory until the existing Apply |

## Sources

### Primary (HIGH confidence)
- `~/.cargo/registry/src/*/egui-0.36.1/src/containers/scroll_area.rs` (wheel gate ~1215, scroll_delta take ~1094, offset override 743, `ScrollSource` 192-216, drag response 831), `interaction.rs` (drag persistence, Escape, thresholds), `hit_test.rs` (`hit_test_on_close`), `response.rs` (`drag_started/dragged/drag_stopped`), `context.rs` (`dragged_id`, `drag_stopped_id`, `stop_dragging`), `ui.rs` (`scroll_with_delta_animation`, `set_opacity`), `painter.rs` (`multiply_opacity`), `layers.rs` (`allow_interaction`), `containers/area.rs` (`interactable`, `order`, `fixed_pos`), `input_state/mod.rs` (`smooth_scroll_delta`, `stable_dt`, `focused`, `consume_key`)
- Repo: `src/overlay/rule_list.rs`, `virtual_list.rs`, `rules.rs`, `app.rs`, `config_view.rs`, `config_state.rs`, `src/filter/structure.rs`, `divider.rs`, `src/models/config.rs`, `src/skin/widgets.rs`, `slice.rs`, `textures.rs`, `screenshot.rs`, `Cargo.lock`, `.planning/phases/49-*/49-CONTEXT.md`, `49-UI-SPEC.md`, `.planning/REQUIREMENTS.md`

### Secondary / Tertiary
- none (no web sources were needed; every claim above was read from source in this session)

## Project Constraints (from CLAUDE.md)

- No new dependencies (phase constraint); egui overlay conventions: every model mutation is returned as an action and applied after the frame (`RowAction` / `ConfigAction`); views never mutate the model.
- Tests pin `FERRISFILTER_HOME` / `FERRISFILTER_GAME_DIR` on every spawn (`tests/isolation_guard.rs`); lib tests never resolve the real folder.
- Overlay UI is "not texty": no new captions; tooltips only. The only new string is `Drag to reorder`; reuse `Clear the search to reorder`.
- Milestone close-out rules: do not add changelog entries or README edits during the milestone; a bug introduced and fixed inside the milestone is never mentioned outside `.planning/`.
- CLAUDE.md must be updated for the new behaviour (arrows removed from rule and style rows, section headers keep theirs, `--drag` screenshot flag, `Skin.drag_handle`, art files in `assets/custom/`), keeping the "Repository layout" reference accurate; docs prose wrap at 70 chars applies to README/CHANGELOG only.
- Art stays under the author's spellings (`icon_reoder_*`, `*-palllet.ase`, `button-prassed-pallet.ase`).
- Escape/drag interplay and window hide behaviour must respect the single-instance / visibility state machine in `visibility.rs` (no branching logic in `app.rs` beyond calling a cancel).

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH, no new packages; versions read from `Cargo.lock`.
- Architecture: HIGH for egui behaviour (source read), MEDIUM for shadow rendering and the style-row shadow (spike S4/S5).
- Pitfalls: HIGH; items marked A1-A5 verified by spike.

**Research date:** 2026-10-23
**Valid until:** until egui is bumped (re-check the `dragged_id()` wheel gate and `scroll_with_delta_animation` on upgrade); otherwise stable.
