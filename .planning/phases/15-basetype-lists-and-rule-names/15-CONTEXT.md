# Phase 15: BaseType Lists and Rule Names - Context

**Gathered:** 2026-09-08
**Status:** Ready for planning

<domain>
## Phase Boundary

Two editor features the author asked for on 2026-09-08, explicitly **before v0.4 closes**:

1. **Base-type matching becomes expressible in the overlay.** A rule can carry a `BaseType` list
   entered item by item, and `class` is demoted from an always-present picker to an optional
   condition like every other. Turning `class` off is what makes a base-type-only rule possible —
   the author's hand-written target has no `Class` line at all:

   ```
   Show # Change Bases
     Rarity == Normal
     BaseType == "Heavy Belt" "Utility Belt" "Armoured Cap" "Sacred Focus"
   ```

2. **A rule can carry an optional name**, shown in the overlay's row header and emitted as the
   block comment (`Show # rule 0 - Exceptional bases`), so rules are identifiable at a glance.

**This is overlay work, not model or emitter work.** Both underlying fields already exist, are
already serialised, and are already emitted — see `<code_context>`. Nothing in this phase should
need to touch `src/emitter.rs` or `src/models/config.rs`, and if a plan proposes to, that is a
signal something has been misread.

**Out of scope:** any change to the economy/pricing sections, the style vocabulary, or
`config.json → universal.filter`. The oracle stays byte-for-byte with zero fixture re-baselining —
the author's standing acceptance bar for the whole v0.4 milestone.

</domain>

<decisions>
## Implementation Decisions

### Base-type entry (R1)

- **D-01: Entry is item by item, not one big text box.** Author: *"the only thing I want is to
  have an easy way to type item by item instead of a single text box with all entries, should
  also be able to remove individual items anywhere in this list."* So the control is a list of
  the base types already on the rule, each with its own remove affordance, plus a single-line
  field that appends one entry at a time. A multi-line textarea holding the whole list is
  explicitly rejected — removing one name from the middle of it would mean text surgery.
- **D-02: Removal is per item, at any position.** Not "clear all", not "remove last".
- **D-03: Committing an entry is `Enter` or an add button**, and commit clears the field so the
  next name can be typed straight away. Typing four bases in a row should not require reaching
  for the mouse between them.
- **D-04: Claude's discretion on the rest of the entry mechanics** (author: *"1 is up to you"*).
  Locked below in *Claude's Discretion* so downstream agents do not re-open it.

### Typos and validation (R1)

- **D-05: No base-name validation. Accept what is typed, verbatim.** Author: *"for now we don't
  handle typos, the game already handles them and warns the user, so for now it's up to the user
  to correct them."* PoE2 itself reports an unknown base when the filter loads, which is a better
  oracle than anything this repo could ship — there is no base-name list in the codebase, and
  vendoring one from poe2db would go stale every league. **Do not add a validation layer, a
  spell-check, a warning icon, or a known-bases fixture.** Trimming whitespace, dropping empty
  entries and ignoring exact duplicates are hygiene, not validation, and are allowed.

### `class` as an optional condition (R1)

- **D-06: `class` gets the same independent off switch every other condition has** (EDIT-08 /
  D-14 from Phase 14). It joins the uniform enable-toggle-plus-value vocabulary rather than
  getting a bespoke treatment.
- **D-07: The class picker is hidden entirely while the toggle is off, and appears when ticked.**
  Author: *"having class [un]ticked to not show the class options, that part can expand once
  ticked by the user."* Not greyed out, not collapsed-but-present — absent. This is the point of
  the feature: it is what makes a base-type-only rule readable in the editor and what makes a new
  rule open clean.
- **D-08: Unticking `class` may discard the class list.** Author: *"unchecking class can destroy
  the list, but i'm not too worried about this so either way works for now."* Given the choice,
  **discard** — it matches what every other off switch already does (`clear_sockets`,
  `clear_quality`, `clear_rarity`, `clear_defence_group`, `clear_unidentified_tier` all set the
  field to `None`). An off switch that secretly retains hidden state is the inconsistency, not the
  data loss. `edit::set_class` already collapses a fully-deselected list to `None`, so this is the
  same end state reached by a different route.
- **D-09: A new rule still has nothing checked.** Author: *"What I want is for a new rule to have
  nothing checked (as it does now)."* D-07 from Phase 14 stands unchanged — a new rule is
  `{ "style": "unchanged", "when": {} }`. What changes is only that its body now opens with no
  class picker expanded, because `class` is off like everything else.

### Rule names (R2)

- **D-10: The row header format is `<index> <style>[ - name]`** — the bracketed part present only
  when the rule has a name. Author's own notation, verbatim. Note this **keeps the style** rather
  than replacing it with the name (Claude's initial leaning, corrected by the author). The
  `[DISABLED]` marker the header already appends is unaffected.
- **D-11: No auto-fill. A name is empty until typed.** Author: *"no auto-fill, a rule name can be
  'unticked' and not used."* A pre-filled `"rule 12"` would go stale the instant the rule is
  reordered, and the filter's block comment would then be actively wrong. The emitter's existing
  fallback already derives `rule {idx}` **at emit time**, so an unnamed rule is always labelled
  correctly without storing anything.
- **D-12: The name has an off switch like everything else**, and unticking it means no name is
  used. For consistency with D-08 and D-14, unticking clears the stored name rather than retaining
  hidden text.
- **D-13: A name is a label, never an identity.** `RuleId` (session-only) stays the identity and
  the array index stays the precedence — `PITFALLS.md` §9. Nothing may key off a name: not the
  expansion state, not delete confirmation, not reordering, not dedup. Two rules with the same
  name is a legal, unremarkable state.

### Claude's Discretion

The author explicitly deferred D-04 (base-type entry mechanics beyond "item by item, removable
anywhere"). These are locked here so planning does not reopen them; the author can still veto any
of them on sight.

- **D-14: The add field strips surrounding double quotes and splits a pasted quoted list.** Typing
  `Heavy Belt` and pasting `"Heavy Belt" "Utility Belt"` should both do the obvious thing. The
  author has been hand-writing these lists in filter syntax and will paste from filters they
  already wrote; making them delete quote characters by hand would be a needless papercut. This is
  string hygiene on input, **not** the validation D-05 forbids.
- **D-15: Base types render in the collapsed summary with an explicit label**, e.g.
  `Bases: Heavy Belt, Utility Belt +2 more`. Today `summarize_list` renders base types as a bare
  name list, structurally identical to a class list; with `class` off and base types on, a
  summary of bare names is ambiguous about which condition it describes. Existing truncation
  (`MAX_CLASSES_SHOWN`) applies unchanged.
- **D-16: The base-type control sits directly after the class control** in the rule body, and the
  name field sits at the very top, above `style`. Class and base type are the two "which item is
  this" conditions and belong together; a name labels the whole rule and is not a condition at
  all. Body order is already independent of emit order, so this constrains nothing downstream.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase definition
- `.planning/ROADMAP.md` § "Phase 15: BaseType Lists and Rule Names" — goal and its five success
  criteria, including the two guard criteria (unnamed rules keep the derived label; a name never
  becomes an identity).
- `.planning/STATE.md` § "Phase 15 Source Material — the author's two requests" — the author's
  request as originally given, plus the groundwork verified before this phase was scoped.

### What this phase builds on
- `.planning/phases/14-rule-editor-apply/14-CONTEXT.md` — D-01..D-19 are all still binding.
  Especially **D-14** (every condition has an independent off switch — the pattern `class` and
  `name` now join), **D-18** (pure logic in the library, presentation at the edge), **D-19**
  (identity survives reordering), and **D-02 as annotated SUPERSEDED** (edits are in-memory;
  Apply is the single writer, persisting the store *then* compiling from it).
- `.planning/phases/14-rule-editor-apply/14-04-SUMMARY.md` — what actually shipped, including the
  eleven-commit live-use fix stream that is not described by any plan.
- `.planning/phases/14-rule-editor-apply/14-MANUAL-VERIFICATION.md` — twelve approved in-game
  checks; the shape any Phase 15 manual checklist should follow.
- `.planning/research/PITFALLS.md` — §8 (GUI has no test story; keep logic in the library),
  §9 (**ordinal identity is fragile — the trap D-13 exists to avoid**), §10 (option lists must
  never be hardcoded), §4 (do not let the GUI regress the tested core).

### The exact emitted forms these controls must produce
- `CLAUDE.md` § "`when` condition vocabulary" — the `baseType` row specifies
  `BaseType == "Base Name" "Base Name" ...`, and the `class` row specifies the **bare, match-any**
  `Class "ClassName" ...` form. Note the asymmetry: base types take `==`, classes do not.
- `CLAUDE.md` § "Authoring conventions" — `Show # What it matches (kebab-slug)` is the house style
  for block comments, which is what a rule name becomes.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets — the groundwork is already in place

Verified by reading the code on 2026-09-08, before this phase was scoped. **Both features already
have their model and emitter halves built:**

- **`When::base_type: Option<Vec<String>>`** — `src/models/config.rs:341`, already
  `skip_serializing_if`, already round-trips.
- **The emitter already writes it** — `src/emitter.rs:418` produces
  `BaseType == "A" "B"` from that field.
- **`Rule::name: Option<String>`** — `src/models/config.rs:373`.
- **The emitter already uses it** — `src/emitter.rs:336-345` computes
  `let derived_label = format!("rule {}", idx); let label = rule.name.as_deref().unwrap_or(&derived_label);`
  then writes `{Show|Hide} # {label}`. The author's desired
  `Show # rule 0 - Exceptional bases` is exactly a `name` of `"rule 0 - Exceptional bases"`.
  **D-11's fallback requirement is already satisfied by this code** — the phase must not break it.
- **`RuleRow::name: Option<String>`** — `src/overlay/view_model.rs:34`. Already derived from
  `Rule.name`; simply never rendered. `row_header_text` in `src/overlay/app.rs:785` currently
  formats `{ordinal:02}  {style}{marker}` and ignores it. D-10 is closer to wiring than building.
- **`edit::set_class`** — `src/overlay/edit.rs:240` already collapses a fully-deselected list to
  `None` rather than `Some(vec![])`, guarding against a malformed bare `Class` line. D-08's
  discard lands on the same state.
- **The uniform enable-toggle pattern** — `render_rarity`, `render_defence`, `render_sockets`,
  `render_quality`, `render_corrupted` and `render_unidentified_tier` in
  `src/overlay/rule_editor.rs` all follow the same `circular_toggle` + value shape (Phase 14's
  RESEARCH Pattern 2). `class` and `name` should adopt it rather than invent anything.

### Established Patterns
- **Pure mutation in `src/overlay/edit.rs`, presentation in `src/overlay/rule_editor.rs`.**
  `edit.rs` is framework-free — its only mention of `egui` is a doc comment, and the plans carry
  greps enforcing that. New setters (`set_base_type` / `clear_base_type` / `set_name` /
  `clear_name`, or whatever shape planning settles on) belong there with plain `#[test]`s.
- **Option lists come from the config, never a literal** (`PITFALLS.md` §10). This does **not**
  apply to base types — there is no config list of base names and D-05 forbids inventing one.
  Base types are free text by design; classes remain config-sourced.
- **Byte-for-byte oracle.** `artifact_guard` + `generate_snapshot` green with zero re-baselining,
  held across Phases 11-14.
- **Every control returns `bool` for "changed this frame"**, which `render_rule_body` ORs together
  and the caller feeds into the dirty flag driving Revert/Apply.

### Integration Points
- `src/overlay/rule_editor.rs::render_rule_body` — the fixed vertical stack every control lives
  in; two new renderers slot in per D-16.
- `src/overlay/app.rs::row_header_text` — D-10's one-line change, plus whatever the name field
  needs from the row loop.
- `src/overlay/view_model.rs::summarize` — D-15's label; note `base_type` is already summarised
  there, just unlabelled.
- `CLAUDE.md` — the overlay section will need the two new controls described once they exist, and
  its current claim that *"Every edit autosaves to `dynamic.json` immediately"* is **already stale**
  (D-02 was superseded in Phase 14). Worth correcting in passing.

</code_context>

<specifics>
## Specific Ideas

- **The motivating rule, in the author's own words** — they have been hand-writing this into
  filters and want it expressible in the editor:

  ```
  Show # Change Bases
    Rarity == Normal
    BaseType == "Heavy Belt" "Utility Belt" "Armoured Cap" "Sacred Focus"
  ```

  Note what it does *not* have: a `Class` line. That absence is the requirement.

- **The name format, in the author's own words:** `Show # rule 0 - Exceptional bases`, and a row
  header of `<index> <style>[ - name]`.

- **Why names at all:** *"this should make the rules more easy to read on the editor."* The
  editor is the primary audience; the filter comment is the side effect. If a design choice has to
  favour one, favour legibility in the stripe.

</specifics>

<deferred>
## Deferred Ideas

- **Base-name validation / autocomplete against a known base list.** Explicitly declined for now
  (D-05), not merely postponed — the author's reasoning is that the game already warns on unknown
  bases. If it is ever revisited, the blocker to solve first is where a base-name list comes from
  that does not go stale each league.
- **`EDIT-09`'s bookkeeping lag — CLOSED 2026-09-08, after this file was written.** It was
  checkbox-unticked and `Pending` in the coverage table although Phase 14 shipped and manually
  verified it. Fixed in `864cbee` while adding EDIT-10/EDIT-11, along with EDIT-03 still naming
  the `unidentified` style that became `kite`. Left recorded here rather than deleted so the
  trail from "noticed during discussion" to "fixed" stays legible.

</deferred>

---

*Phase: 15-basetype-lists-and-rule-names*
*Context gathered: 2026-09-08*
