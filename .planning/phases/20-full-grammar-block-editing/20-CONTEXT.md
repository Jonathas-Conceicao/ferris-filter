# Phase 20: Full-Grammar Block Editing - Context

**Gathered:** 2026-10-12
**Status:** Ready for planning
**Mode:** Smart discuss (autonomous, all v0.5 phases discussed upfront)

<domain>
## Phase Boundary

A rule's expanded body can edit every condition and every action in GGG's documented grammar,
with no read-only conditions (author acceptance bar). Named styles can be applied to a rule and
stay linked to `styles.json` through FerrisFilter's metadata comment, so style changes flow into
the rule on Apply. Requirements: EDIT-01, EDIT-02, EDIT-06, STYLE-03.

</domain>

<decisions>
## Implementation Decisions

### Condition editing (intuitive and accessible, author emphasis)
- **+ condition** picker listing every condition keyword from Phase 16's keyword table, in
  readable groups. Each condition row has three parts:
  - the keyword;
  - an operator dropdown offering only the operators valid for that keyword, with plain-English
    labels as in v0.4 ("At least", "Exactly", …);
  - a value control chosen by value type.
- The same keyword may appear more than once, e.g. `ItemLevel >= 70` and `ItemLevel <= 80`.
- **Value controls (pickers and sliders, not raw text, where the value has structure):**
  - **Class:** v0.4's grouped class picker (JEWELLERY etc. from `config.json`'s
    `jewelleryClasses`; never hardcode groups in the GUI).
  - **BaseType and other quoted string lists:** v0.4's list entry (type + Enter / **+ ADD**,
    per-entry Remove, quoted-paste splitting).
  - **Rarity:** a dropdown, with multi-value support for `Rarity Normal Magic Rare`.
  - **Numeric conditions** (ItemLevel, DropLevel, AreaLevel, Quality, Sockets, StackSize,
    WaystoneTier, Base defences, GemLevel, UnidentifiedItemTier, …): sliders bounded by sensible
    per-keyword ranges, with an exact-number entry alongside.
  - **Booleans** (Corrupted, Mirrored, Identified, …): a toggle.
  - **Enums:** dropdowns.
  - **Socket strings** (e.g. `5GGG`) and mod lists (`HasExplicitMod >=1 "…"`): structured fields
    that render valid grammar.
- **v0.4's three-checkbox defence picker is replaced** by the generic BaseArmour, BaseEvasion
  and BaseEnergyShield rows, which use the numeric slider controls.

### Action editing (EDIT-02)
- **Colours** (SetTextColor, SetBorderColor, SetBackgroundColor): egui colour picker with alpha.
- **SetFontSize:** slider, 1–45.
- **MinimapIcon:** dropdowns for size, colour and shape.
- **PlayEffect:** colour, plus a `Temp` toggle.
- **Sounds:**
  - `PlayAlertSound` / `PlayAlertSoundPositional`: id and volume sliders.
  - `CustomAlertSound` (and `CustomAlertSoundOptional`): file name and volume.
- **DisableDropSound / EnableDropSound** (and the other drop-sound toggles in the grammar):
  toggles.

### Editing rules
- Every edit renders valid grammar. Only the edited value's lines change (PARSE-04). An existing
  operator is never normalized (e.g. `Class "x"` never becomes `Class == "x"`) unless the user
  changes it.

### Styles on rules (STYLE-03, EDIT-06)
- **Applying a style** to a rule writes a `#ff:style <name>` line into the rule's FerrisFilter
  metadata comment run (Phase 16 convention: header-style comment, not a tag inside the
  `Show #` text). It sets the rule's style actions from `styles.json`, and the style's Show/Hide
  overrides the rule's own (Phase 17).
- **On Apply, a rule's style actions are rewritten from its style** whenever its `#ff:style`
  line names a known style. This is how style edits in the config tab reach existing rules.
- **Hand-edited actions on a styled rule are overwritten by the style on Apply** (author
  decision: "if the style comment is not also hand-edited, we can just rewrite it"). Removing or
  changing the `#ff:style` line is how a rule leaves its style.
- The UI must make this visible: a styled rule's style-owned actions show as coming from the
  style, with an obvious way to remove the style when the user wants custom actions.
- A `#ff:style` naming an unknown style (deleted or typo) is reported. The rule keeps its current
  actions, and no rewrite happens for it.

### Claude's Discretion
- The grouping of the condition picker, slider ranges per keyword, widget layout within the narrow
  stripe, and how the accordion body is restructured from v0.4's fixed control set into the
  data-driven one.

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `rule_editor.rs`:
  - class picker groups, the base-type list widget, and the per-condition off-switches;
  - plain-English operator labels;
  - the Phase 15 name toggle and field (now `#ff:name`).
- `edit.rs` mutators (`set_class` collapsing an empty list to `None`, etc.). Their lessons apply:
  an empty value list must never emit a malformed bare keyword line.

### Established Patterns
- The off-switch per condition (v0.4 EDIT-08) hid the control when off. The generic model
  replaces it with adding and removing condition rows.
- Base names are not validated (PoE2 warns at load).

### Integration Points
- Phase 16's keyword table drives both the picker and the value controls, and Phase 16's
  line-patching API applies edits. Phase 17's `StyleSet` provides the style list and actions.

</code_context>

<specifics>
## Specific Ideas

- The author's priority is **a more intuitive and accessible UI**: pickers for categorical values,
  sliders for numbers, and structured controls everywhere a value has structure.

</specifics>

<deferred>
## Deferred Ideas

- Raw-text block editing ("Edit as text"): backlog 999.10 (author: not now).

</deferred>
