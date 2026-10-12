# Phase 19: Overlay Rule List Over Parsed Blocks - Context

**Gathered:** 2026-10-12
**Status:** Ready for planning
**Mode:** Smart discuss (autonomous, all v0.5 phases discussed upfront)

<domain>
## Phase Boundary

The overlay's rule list shows a parsed filter (Phase 16), with Phase 18 as the store. The user can
open any filter's blocks as rules and do the following:
- add, delete, duplicate, reorder, enable and disable rules;
- switch a rule between Show, Hide and Minimal, and toggle `Continue`;
- search the list.

Apply writes and Revert discards. The economy section appears only as a locked summary.

Requirements: EDIT-03, 04, 05, 07, 08; ECON-02. Full per-condition and per-action editing is
Phase 20. The config tab, including choosing a different filter, is Phase 21.

</domain>

<decisions>
## Implementation Decisions

### Rule naming (author decision: FerrisFilter's own pattern only)
- A rule's displayed name comes **only** from FerrisFilter's metadata comment run above the block
  (`#ff:name …`, Phase 16's convention).
- Foreign header text, such as NeverSink's `Show # %D4 $type->currency $tier->s !slug` and
  poe2filter.com slugs, is **never** shown as the name. The author wants no imported jargon in
  the editor.
- An unnamed rule shows a derived label: its index, Show/Hide/Minimal, and a short condition
  summary (e.g. `12  Show · Currency · BaseType ×14`), in the same spirit as v0.4's
  `rule <index>`.
- Naming a rule writes or updates its `#ff:name` line. The v0.4 name toggle and field carry over,
  retargeted to the `#ff:name` line.
- Foreign comments on the block are preserved verbatim on disk. They are just not used as the
  name.

### Large filters
- A **flat list**, with search and row virtualization: only visible rows are drawn, and row
  summaries are cached after parsing.
- **No grouping by foreign banner comments**, because tool-specific comment styles aren't parsed
  (Phase 16).
- It must stay responsive at about 460 blocks, the size of the NeverSink sample (EDIT-07).
- **Search** matches the rule's `#ff:name`, the derived label, and Class / BaseType values.

### List operations
- **Reorder:** up and down arrows, as in v0.4 (no drag-and-drop). Moving a rule across a block
  that has `Continue` shows a warning indicator but is not blocked.
- **Enable/disable:** the v0.4 circular toggle. Disabling comments the block out, and enabling
  uncomments it (Phase 16 semantics).
  - Commented-out blocks in a loaded filter appear as disabled rules.
  - A disabled rule whose commented lines are not valid grammar shows as disabled with an
    "invalid, can't enable" reason. It can still be moved or deleted, but enabling is refused.
- **Add / Delete / Duplicate:**
  - **+ ADD RULE** appends an empty, valid rule before the economy section.
  - Delete keeps v0.4's inline confirm.
  - Duplicate copies a block and gives it a fresh identity.
- **Show/Hide/Minimal and Continue** are set per rule (EDIT-03). If the rule has a style tag, the
  style's action wins on Apply (Phase 17 decision), and the UI should make that visible.
- **Enabled blocks with grammar errors** (Phase 16) are flagged on their row, and Apply is
  refused until they are fixed or disabled.

### Economy section and Apply
- **Economy footer row:** "+ N locked rule blocks (economy)", with **Add economy section** when
  absent and **Remove** when present (ECON-02 + the ECON-04 UI). The blocks themselves are never
  listed or editable.
- **Apply and Revert** keep v0.4's model (EDIT-08):
  - edits are held in memory, and Apply is the single writer;
  - Revert re-parses the last applied text;
  - the stale indicator and the "Options → Game → Item Filter → Refresh" reminder after Apply both
    stay.
- **Apply's safety outcomes** come from Phase 18 and are surfaced in the stripe:
  - changed on disk;
  - damaged markers;
  - invalid enabled block.

### Claude's Discretion
- Virtualization technique (egui `ScrollArea::show_rows` or equivalent), the summary text format,
  and how stable rule identity carries over from v0.4's session-only `RuleId`.

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `src/overlay/rule_editor.rs`: the accordion rows, arrows, toggle and confirm-delete.
- `src/overlay/edit.rs`: `WorkingRules` and the session `RuleId`, which survives reordering.
- `src/overlay/view_model.rs`: `RuleRow`, `row_header_text`, `summarize_list`.
- `src/overlay/apply.rs`: `ApplyState`/`ApplyOutcome` and the stale indicator.
- `src/overlay/widgets.rs`: arrow and toggle widgets.

### Established Patterns
- **Deferred actions:** row controls record a `RowAction` during render, and the mutation happens
  once after the loop.
- **Shutdown:** Quit is deferred until after render.
- **Labels:** plain-English labels instead of symbols.

### Integration Points
- The overlay's model changes from `Vec<Rule>` (JSON store) to the parsed filter (Phase 16)
  plus the Phase 18 store/apply functions.
- The accordion's expanded body is reworked in Phase 20. This phase can keep a minimal body, or
  the v0.4 body where it maps cleanly.

</code_context>

<specifics>
## Specific Ideas

- Keep the v0.4 look: Cinzel header, Dracula palette stripe, `kite` naming lessons. No visual
  redesign beyond what the new list needs.

</specifics>

<deferred>
## Deferred Ideas

- Grouping by banner/section comments.
- Drag-and-drop reordering.

</deferred>
