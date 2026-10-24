# Phase 44: Auto Detect Styles - Context

**Gathered:** 2026-10-23
**Status:** Ready for planning
**Mode:** Smart discuss (autonomous run 43-50; author answered every question up front in prose)

<domain>
## Phase Boundary

A CONFIG tab **Auto Detect** button turns the styling already in the loaded filter into named
styles: rules matching an existing style (built-in or custom) get tagged with it, each remaining
distinct style becomes a new `Custom Style N`, and its rules get tagged — all as ordinary pending
edits until Apply. Requirements DETECT-01..07. Depends on Phase 43 (alpha is part of equality).

</domain>

<decisions>
## Implementation Decisions

### Matching (author)
- Every rule **outside the economy section** is read: its action lines plus Show / Hide are parsed
  into the full `Style` struct, and compared on **every parameter** (full equality). A font size off
  by 3 is a different style.
- Order of matching: built-ins first, with their **current pending values** (user edits included),
  then existing custom styles, then new styles.
- A bare `Show` (no action lines) matches `unchanged`.
- A bare `Hide` matches no built-in (`hidden` also sets font 21 and DisableDropSound) — it becomes a
  custom style with Hide visibility, like any other distinct style.
- Equality is on **in-game values**, not text: a 3-value `SetBackgroundColor` means alpha **240**
  (game default), 3-value text / border mean **255**; a missing volume means the game default
  (`PlayAlertSound` 50, `CustomAlertSound` 100). So `SetBackgroundColor 40 42 54` becomes a style
  with background alpha 240.
- Apply rewrites a tagged rule from its style, as it does every styled rule today; that may normalise
  the rule's form (4-value colours, explicit defaults, line order). This is accepted.

### What stays Custom (author)
- Rules whose actions no style can express stay Custom: repeated lines of one action family (an
  earlier line overridden by a later one), `CustomAlertSoundOptional`, the `...IfAlertSound`
  keywords, out-of-range values, and anything else the style lines cannot show exactly (reuse the
  existing "leftover" classification of the Custom style editor).
- **Trailing comments on action lines do not block a match**; they are dropped when Apply rewrites
  the rule. No warning. Leave a code comment where they are dropped saying this is intentional, in
  case it is changed later.

### Scope (author)
- Disabled rules are included; a `# ff:group` counts as one rule (the whole group gets the tag);
  rules already tagged with a style are skipped; commented-out blocks that are not valid grammar
  (shown `can't enable`, read-only) are skipped; economy rules are never touched.

### Pending-edit model (author)
- Auto Detect never writes to disk. New styles are pending CONFIG edits and tags are pending RULES
  edits, shown as unapplied like any other edit (tab ` *`, indicator, Quit popup); Apply writes them
  through the normal all-or-none Apply.
- Revert and Reload keep their **usual per-tab behaviour**. Consequence accepted: reverting CONFIG
  after Auto Detect leaves rules tagged to a now-absent pending style; they show the existing
  `unknown style` flag (never blocking) and Apply drops the tag, keeping their actions.

### UI
- The button sits on the **Custom** heading of the Styles section, next to **+ Add style**.
- New styles are named `Custom Style N` with the smallest free N (case-insensitive against all
  pending style names), get the normal random custom-style id, and are appended after the existing
  custom styles in order of first appearance in the file. They are then edited, previewed, renamed
  and deleted like any custom style.
- Status line, terse: `Auto Detect: 3 styles, 41 rules tagged`; nothing to do:
  `Auto Detect: nothing new`. A second click with nothing new changes nothing.

### Claude's Discretion
- Where the detection logic lives (a pure module, e.g. `style_detect`, wrapping `rule_style_view` /
  `style_leftovers`), and how the batch is applied to the CONFIG and RULES models (research
  recommends one batch: add pending styles → re-sync the rules model's style set → tag rules, not
  per-rule `ApplyStyle`).

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `rule_style_view()` and `style_leftovers()` in `src/overlay/body.rs` — parse a rule's action lines
  into a `StyleView` and list lines that cannot be shown exactly (the Custom style editor uses them).
  A `StyleView::to_style()` step is the missing piece.
- Custom style creation / id generation (+ Add style) in `src/overlay/config_state.rs`.
- Rule tagging (`# ff:style <id> <name>`) in the RULES model (`src/overlay/rules.rs`,
  `src/styled.rs`).

### Established Patterns
- One shared Apply (`src/overlay/apply_all.rs`), pending edits held in memory per tab, status line
  messages via the shell's status model.

### Integration Points
- CONFIG view `src/overlay/config_view.rs` (Custom heading), RULES model, the unknown-style flag
  path, the unapplied indicator and Quit popup counting.

</code_context>

<specifics>
## Specific Ideas

- Research `.planning/research/SUMMARY.md` pitfall 2 (equality traps) and 3 (per-tab Revert matrix:
  {Revert Rules, Revert Config, Reload, Quit} × {before, after Auto Detect}); table-driven tests.

</specifics>

<deferred>
## Deferred Ideas

- Auto Detect for the economy section's custom-styled blocks — a future milestone (author).

</deferred>
