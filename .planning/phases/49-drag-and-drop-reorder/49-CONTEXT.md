# Phase 49: Drag-and-Drop Reorder - Context

**Gathered:** 2026-10-23
**Status:** Ready for planning
**Mode:** Smart discuss (autonomous run 43-50; author answered every question up front in prose)

<domain>
## Phase Boundary

Rules (RULES tab) and custom styles (CONFIG style table) are reordered by dragging a handle drawn
with the author's art; the up / down arrows on rule and style rows go away (section headers keep
theirs). Requirements DND-01..05. Builds on Phase 48's redrawn rows.

</domain>

<decisions>
## Implementation Decisions

### Handle (author)
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

### Drag behaviour (author)
- Press the handle to start; the source row is greyed in place; an expanded row collapses when its
  drag starts.
- Shadow: the collapsed row at ~60% opacity, following the cursor vertically, aligned to the list
  horizontally.
- Drop marker: a gold line between rows.
- Mouse wheel scrolls during a drag; within ~40pt of the top / bottom edge it auto-scrolls, faster
  closer to the edge (time-based, repaint every frame while dragging).
- Release on a valid slot moves the row as **one pending edit** (Apply writes, Revert undoes);
  release outside the list or Escape cancels (also cancel if the window hides / loses focus).

### Rules kept from the arrows (author)
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

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `src/overlay/rule_list.rs` (row drawing, arrows, `RowAction`), `src/overlay/virtual_list.rs`
  (offsets), `src/filter/structure.rs` (moves), `src/overlay/config_view.rs` (style table Order
  arrows), `src/skin/textures.rs` (embedding art; `assets/custom/titlebar.png` is the precedent for
  custom art).

### Established Patterns
- `move_rule` swaps one step and toggles the Continue flag per step — not reusable for multi-gap
  moves as is.

### Integration Points
- RULES list, CONFIG style table, pending edits / Revert, search state.

</code_context>

<specifics>
## Specific Ideas

- Author-drawn handle art (three states) and palettes in `assets/custom/`.

</specifics>

<deferred>
## Deferred Ideas

- 999.16 (move to top / bottom, into a named section menu, + Add rule insert position) stays in the
  backlog (author).

</deferred>
