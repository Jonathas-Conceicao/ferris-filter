# Phase 31: RULES Tab on the Skin - Context

**Gathered:** 2026-10-16
**Status:** Ready for planning

<domain>
## Phase Boundary

Redraw the RULES tab (rule list + expanded rule editor) with `src/skin` controls inside the Phase 30
shell, losing no v0.5 capability, and make + Add rule insert at the top of the file. Dividers are
Phase 32 (no + Add section here).

</domain>

<decisions>
## Implementation Decisions

### Rule rows
- Follow the ff-proto RULES look: round enable check, index, Show/Hide, name (`# ff:name` only),
  second line with the condition summary, flag icons (from Phase 30's less-texty treatment),
  `Continue` tag and up/down arrows on the right; `styled: <name>` truncation and the `+N` flag badge
  budget rules from v0.5 still apply (adapt numbers to the new width).
- Search box + `+ Add rule` in the toolbar row as in the prototype. Stays responsive on the
  ~500-block NeverSink/FilterBlade sample (virtualized list kept).

### Editor
- Expanded editor is two columns: labels left, controls right (as the prototype's expanded rule).
- Migrate through the existing wrappers (`themed_slider`, `circular_toggle`, `slide_switch`,
  `toggle_chip`, dropdowns, text fields) to skin widgets so every editor path comes along: all
  conditions/actions, operators, Between, Defences / Simple Defences chips, grouped rules
  (`# ff:group`), raw rows, quoted lists, class picker, style picker (Custom + presets), Show/Hide,
  Continue, Duplicate, Delete (inline confirm may become the skinned popup).
- Colour actions keep their current picker until Phase 33 delivers the shared skinned picker.

### Add rule
- `+ Add rule` inserts the new empty `Show` rule as the first block of the file, always outside every
  section (Phase 32 will add sections; the insertion point stays "top of file, before any divider").

### Claude's Discretion
- Exact spacing, summary truncation, icon set, how the per-row flag tooltips read.

</decisions>

<code_context>
## Existing Code Insights

- `src/overlay/rule_list.rs` (1419), `rules.rs` (2682), `body.rs` (5812 — editor body),
  `conditions.rs`, `classes.rs`, `widgets.rs` (wrappers), `virtual_list.rs`, `grammar.rs`.
- Add-rule currently appends before the economy section (`src/filter/edit.rs` / `structure.rs`).
- Skin widgets in `src/skin/widgets.rs`; prototype composition in `src/bin/ff-proto.rs`.

</code_context>

<specifics>
## Specific Ideas

- Reference: `ff-proto-rules.png` in `.planning/quick/261016-1de-.../`.

</specifics>

<deferred>
## Deferred Ideas

- Moving rules many positions at once — backlog 999.16.

</deferred>
