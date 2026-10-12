# Phase 21: Overlay Configuration Tab - Context

**Gathered:** 2026-10-12
**Status:** Ready for planning
**Mode:** Smart discuss (autonomous, all v0.5 phases discussed upfront)

<domain>
## Phase Boundary

A configuration tab in the overlay, separate from the rule editor, where the user can:
- choose which game-folder filter is being edited;
- edit named styles (`styles.json`);
- edit the economy tier cutoffs (`tiers.json`).

The tab has its own Apply and Revert. Requirements: CONF-01..04, STORE-02. There is no load
message (backlog 999.3, online-only).

</domain>

<decisions>
## Implementation Decisions

### Layout
- **RULES | CONFIG** tabs at the top of the stripe. CONFIG has three sections: **Filter**,
  **Styles** and **Tiers**.

### Filter selection (STORE-02)
- A dropdown of the `*.filter` files in the game folder, plus **New empty filter** (STORE-03
  semantics from Phase 18).
- The choice is remembered across launches in a **git-ignored, per-user `ff-settings.json`** at
  the app root, resolved through Phase 18's resolver. The default is `dynamic.filter`.
- Switching filters while the rules tab has unapplied edits asks first; it never silently
  discards them.

### Style editor (CONF-02)
- **List and preview:** styles are listed in file order, each row expanding into an editor, and
  each row shows a preview chip in its text, border and background colours.
- **Editable fields:**
  - action (Show/Hide);
  - text, border and background colours, through the egui colour picker with alpha;
  - font size;
  - alert sound or custom sound;
  - minimap icon (size, colour, shape);
  - beam (colour and `Temp`);
  - drop-sound toggle.
  Hide styles get the full field set (Phase 17).
- **Adding:** a new style needs a unique, non-empty name. Renaming is not supported in v0.5.
- **Deleting:**
  - **Built-ins can't be deleted:** S, A, B, C, D, E, F, `unchanged` and `hidden`.
  - **Other styles can be deleted even while rules use them.** On the next Apply, affected rules
    keep their current actions and their `#ff:style` line is dropped.

### Tier editor (CONF-03)
- One row per tier, S to F, each with a min-value control. The unit is shown read-only.
- Text buffers are kept separate from parsed values. `ValidatedTiers` is the single validator,
  and its errors (e.g. cutoffs out of order) show inline. Invalid cutoffs can't be applied.
- The tab edits the live `tiers.json` only. The committed `prices-fallback.json` /
  `tiers-fallback.json` pair is never touched.

### Apply and Revert (CONF-04)
- Config Apply validates both files, writes both to temp files, then renames both: all or
  nothing.
- It writes **only** `styles.json` and `tiers.json`. The filter is then shown as out of date until
  the RULES tab's Apply regenerates the economy section and style-owned actions.
- Config Revert discards unsaved config edits. Config edits are held in memory, as with the rules
  tab.
- An mtime check against outside edits to `styles.json`/`tiers.json` is the same safety idea as
  Phase 18's guard.

### Claude's Discretion
- Widget layout in the narrow stripe, preview chip design, and the exact wording of the
  out-of-date indicator.

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- egui 0.36 built-ins: `color_edit_button_srgba`, `DragValue`, `ComboBox`. Colours are stored as
  `Color32` (u8 sRGB) so they round-trip exactly.
- `ValidatedTiers` and `TierValidationError` in `src/models/tiers.rs`.
- The v0.4 Apply/Revert/stale-indicator machinery in `src/overlay/apply.rs`.
- The `StyleSet` load/save from Phase 17, and the app-root resolver and atomic write from
  Phase 18.

### Integration Points
- `src/overlay/app.rs` (tab switching), and a new config view module beside `rule_editor.rs`.

</code_context>

<specifics>
## Specific Ideas

None beyond the decisions above.

</specifics>

<deferred>
## Deferred Ideas

- Load message configuration (999.3, online-only).
- Style rename.
- Per-user styles location.

</deferred>
