# Phase 14: Rule Editor + Apply - Context

**Gathered:** 2026-09-06
**Status:** Ready for planning

<domain>
## Phase Boundary

Every dynamic rule becomes editable from the overlay — style, classes, rarity, defences, sockets,
quality, corrupted, each condition independently togglable — plus add, delete, reorder and
enable/disable. An Apply action writes `dynamic.filter` into the game folder and tells the player
to reload it in-game.

**In scope:** the editing controls (EDIT-01..08), Apply (EDIT-09), rule mutation and validation as
pure library functions, and the store/filter persistence split described in D-02 below.

**Out of scope:** editing the style palette itself, the class lists, or the economy tiering — all
pinned this milestone. No new overlay window configuration (Phase 13's geometry, focus semantics
and hotkey are settled). No `sync` from the overlay.

This is the last phase of v0.4.

</domain>

<decisions>
## Implementation Decisions

### Editing surface

- **D-01: Rules expand in place — an accordion, not a detail pane or a separate edit mode.**
  Clicking a rule opens its controls inline; the rows above and below stay visible. Rationale:
  ordinal position *is* precedence in this model, so seeing where a rule sits while changing it is
  part of understanding what the change does. A detail pane or modal edit mode hides that.
  - **The stripe does NOT resize when a rule is expanded.** Controls stack vertically inside the
    expanded row (label above control) rather than sitting side by side, which fits the existing
    460px. This keeps Phase 13's "never sized to the full screen" invariant (SHELL-03 / D-12)
    trivially true instead of something to re-verify, and avoids a dynamic resize interacting with
    the raw-HWND show/hide path.
  - Author's words: *"fine by me, use your best judgment"* — the reasoning above is Claude's, and
    a planner may revisit the layout so long as the no-dynamic-resize constraint holds.

### Persistence — the store and the filter move independently

- **D-02: `dynamic.json` autosaves on every edit; `dynamic.filter` changes ONLY on Apply.**
  > **SUPERSEDED 2026-09-08 — edits stay in memory; Apply writes both files.** After using the
  > editor, the author asked for a Revert button beside Apply and for changes to be held until
  > applied. Apply is now the single writer: it persists `dynamic.json` and then compiles
  > `dynamic.filter` from it, in that order (compiling first would reproduce the stale-Apply bug
  > fixed in 3375ad4). Revert restores the last written state.
  >
  > This reverses the crash-safety argument below, knowingly: a crash or Quit now discards
  > unsaved edits. What replaces it is visibility plus a deliberate discard — the unsaved-changes
  > indicator says the state exists, and Revert is how you throw it away on purpose.
  Author: *"autosave the json, change the .filter only on apply"*. The store is per-user,
  git-ignored state, so losing tuning work to a stray Quit is the worse failure; the filter is what
  the game reads, so it changes only when the player says so.
- **D-03: Because of D-02 the two can diverge, and the UI must say so.** After any edit there are
  saved-but-unapplied changes. The Apply affordance must indicate this state — otherwise "did I
  apply that?" becomes guesswork, and the player blames the filter for not changing when in fact
  it was never written. This is a required part of EDIT-09, not decoration.
- **D-04: Autosave must not corrupt the store.** Writes go through the same atomic
  write-then-rename discipline `dynamic::write_filter` already uses (temp file in the destination
  directory — the repo dir here). A half-written `dynamic.json` would be worse than a lost edit.

### Reordering, adding, deleting

- **D-05: Reordering is up/down arrow buttons per row, not drag-and-drop.** Author: *"arrows"*.
  Ten rules, a narrow scrolling stripe, and egui drag-reorder is fiddly. Arrows always work.
- **D-06: A new rule is appended at the END of the list.** Author: *"add to the end of list, i.e.
  bottom, it doesn't take effect until applied and reloaded in-game by the user so a catch
  all/incomplete rule is a non-issue"*. This is the reasoning that makes the sharp edge safe:
  nothing an incomplete rule does can reach the game until the player deliberately Applies *and*
  reloads in-game. First-match-wins also means a catch-all at the bottom only catches what nothing
  above it matched.
- **D-07: A new rule is `{ "style": "unchanged", "when": {} }`, enabled.** `unchanged` is the
  no-op style (native rarity colour, no font/minimap/beam overrides), so even once applied a
  freshly-added rule changes nothing visually until it is configured. Enabled rather than disabled,
  per D-06's reasoning.
- **D-08: Deleting requires a quick confirm.** Author: *"have a quick confirm for deleting, can
  look like whatever you want"*. Claude's design: an **inline** confirm — the row itself becomes
  `Delete? [Yes] [Cancel]` — rather than a modal dialog, which suits a narrow stripe and needs no
  new window. No undo stack this phase.

### Controls (the EDIT-03..08 vocabulary)

All options come from `config.json`, never hardcoded (`PITFALLS.md` §10 — hand-typed class names
already caused a real in-game defect in v0.2).

- **D-09: Style selector** — a dropdown of the 12 styles currently in `config.json`
  (`S A B C D E F Recycle unchanged hidden unidentified exceptional`), read from the file rather
  than listed in code. Because the selector can only offer real styles, the unknown-style
  generate-time error becomes unreachable from the UI.
- **D-10: Class picker** — a grouped, collapsible checkbox list sourced from `weaponClasses` (12),
  `armourClasses` (6) and `flaskCharmClasses` (3). Groups are collapsed by default; 21 checkboxes
  expanded at once would swamp the stripe.
- **D-11: Rarity selector** — comparator + value, emitting the existing compact forms
  (`>=Magic`, `<=Rare`, `==Rare`, …). Not free text.
- **D-12: Defence picker** — a group toggle plus three checkboxes (armour / evasion / energy
  shield). Ticked emits `> 0`, unticked emits `0`, and with the group off no defence line is
  emitted at all. This is what makes all seven PoE2 armour combinations reachable by ticking three
  boxes instead of choosing among seven options (the author's framing during v0.4 scoping).
- **D-13: Sockets and quality** are minimum-value fields with their own enable toggles; corrupted
  is a control whose unset state omits the condition entirely. **Typed numeric entry is viable** —
  Phase 13's D-06/D-08 settled that focus follows a click into the stripe, so the keyboard reaches
  it. Steppers are still acceptable; this is no longer forced to be mouse-only.
- **D-14: Every condition control toggles off individually, and a disabled control contributes
  nothing to the emitted block.** This is EDIT-08 and it is simply the UI expression of DYN-04 —
  `When`'s fields are already all `Option<…>`, so the model needs no change.

### Apply

- **D-15: Apply compiles the store and writes `dynamic.filter` into the game folder**, reusing
  `dynamic::compile_store` + `dynamic::write_filter` unchanged — the functions Phase 12 built as
  print-free, fully-parameterised callables precisely so a GUI could be their second caller.
- **D-16: Apply states that the filter must be reloaded via Options → Game → Item Filter →
  Refresh.** PoE2 never re-reads a filter on its own; a tool that writes silently reads as broken.
  The exact in-game path is visible in `reference/poe2-options-palette.png` from Phase 13.
- **D-17: Apply failures surface in the stripe**, not a console the player cannot see (the GUI
  binary runs under the Windows GUI subsystem in release builds).

### Layering — unchanged from Phase 13, restated because this phase is where it will be tested

- **D-18: Rule mutation, validation and compilation are library functions covered by unit tests;
  the egui layer holds no logic that can only be exercised by clicking.** `PITFALLS.md` §8. Phase
  13 established three pure modules (`visibility`, `theme`, `view_model`); this phase adds mutation
  and should follow the same shape — a pure `edit`/`mutate` module with plain `#[test]` coverage.
  Roadmap success criterion 8 makes this explicit.
- **D-19: Selection must survive reordering.** `PITFALLS.md` §9: rules are identified by position,
  so a move changes indices. Any "currently expanded rule" state must be updated by the move
  operation itself, or the player silently edits the wrong rule after using an arrow.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase definition
- `.planning/ROADMAP.md` § "Phase 14: Rule Editor + Apply" — goal and its eight success criteria.
- `.planning/REQUIREMENTS.md` § "Rule Editor (EDIT)" — EDIT-01..09.

### What this phase builds on
- `.planning/phases/13-overlay-shell/13-CONTEXT.md` — D-01..D-25, still binding. Especially D-24
  (the sampled PoE2 palette table), D-14 (header plate, `✕` hides), and D-09's SUPERSEDED note
  (the hotkey is now `Ctrl+Shift+Z`).
- `.planning/phases/13-overlay-shell/13-VERIFICATION.md` — what shipped and was verified live.
- `.planning/phases/12-dynamic-rule-store/12-VERIFICATION.md` § "Integration Check" — records that
  `ensure_store`, `compile_store` and `write_filter` are public, print nothing, and take every path
  as a parameter, specifically so the overlay could be their second caller. That is now cashed in.
- `.planning/research/PITFALLS.md` — §8 (GUI code has no test story; keep logic in the library),
  §9 (ordinal identity is fragile under reordering), §10 (class lists must not be hardcoded),
  §1 (PoE2 never reloads a filter by itself), §4 (do not let the GUI regress the tested core).

### Code this phase extends
- `src/overlay/view_model.rs` — `RuleRow { ordinal, enabled, style, name, summary }` and
  `rule_rows(&Config)`. The read-only projection the editor grows out of.
- `src/overlay/store.rs` — the seed-then-load path; autosave (D-02) is its write counterpart.
- `src/overlay/app.rs`, `widgets.rs`, `theme.rs` — the rendering layer.
- `src/dynamic.rs` — `compile_store`, `write_filter`, `ensure_store`, `reset_store`.
- `src/models/config.rs` — `Rule` / `When`; every condition field is already `Option<…>`.
- `config.json` — `styles` (12) and `weaponClasses`/`armourClasses`/`flaskCharmClasses` (12/6/3).
- `CLAUDE.md` § "`when` condition vocabulary" — the exact emitted forms each control must produce.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **`view_model::rule_rows`** already derives display rows from a `Config` as a pure function; the
  editor's per-rule state is the natural extension of it.
- **`dynamic::compile_store` / `write_filter`** are exactly Apply, minus a button. `write_filter`
  already does the atomic write-then-rename with the temp file in the destination directory
  (required — the game folder is on OneDrive, so a cross-volume rename would fail).
- **`theme::poe2_visuals()`** and the widget helpers give the new controls their look for free.
  `#A36C3C` was deliberately reserved during Phase 13 for this phase's Reset — the colour PoE2
  itself uses for "use default".
- **`Rule.enabled: Option<bool>`** and the emitter's disabled-rule skip already exist (Phase 12).
  EDIT-02 is a checkbox over a field that is already modelled, emitted and tested.

### Established Patterns
- **Pure logic in the library, presentation at the edge** — three modules already follow it, with
  greps in the plans forbidding framework types in their signatures.
- **TDD RED/GREEN pairs** were used for the pure modules in Phase 13 and suit mutation logic well.
- **Byte-for-byte oracle** — `artifact_guard` + `generate_snapshot` must stay green with zero
  re-baselining. Held across Phases 11-13; the bar does not relax for the last phase.

### Integration Points
- `src/overlay/app.rs` holds the frame loop every plan in Phase 13 touched; expect it to be the
  serialisation point again.
- `CLAUDE.md` will need the editor described once it exists.

</code_context>

<specifics>
## Specific Ideas

- **The motivating example, from milestone scoping:** *"through the overlay the user can, as an
  example, hide all flask drops and have that overwrite a dynamic.filter."* Roadmap success
  criterion 2 makes it concrete — disabling a single rule covering the flask and charm classes
  removes flask drops from the generated filter, in one click. If that is awkward, the design is
  wrong regardless of what the requirements say.
- **On the defence picker**, the author's original framing: *"The game has 7 types of armour: full
  ES, full Ev, full Ar, the combination of 2 of each, and also in rare cases armour with all 3. The
  picker should allow player to select the exact one of these seven they want. But it's easier to
  pick just selecting the 3 armour types instead of one among all seven combinations."*
- **On safety of adding rules:** *"it doesn't take effect until applied and reloaded in-game by the
  user so a catch all/incomplete rule is a non-issue."* Worth remembering — it is the reason
  several otherwise-scary edit operations need no guard rails.

</specifics>

<deferred>
## Deferred Ideas

- **Undo / undo stack** — not this phase. Delete is guarded by an inline confirm (D-08) instead.
- **Editing the style palette, class lists, or economy tiering from the overlay** — pinned for
  v0.4 by the milestone's own scope.
- **Drag-and-drop reordering** — arrows chosen instead (D-05); revisit only if the rule count grows
  well past ten.
- **Rule search / filtering** — unnecessary at ten rules.
- **A settings file** (hotkey binding, stripe position/width persistence) — still absent; several
  deferred items from Phase 13 are waiting on one.

### Closed, do not reopen
- **Dimming the game behind the stripe** — requested during Phase 13 verification, then declined by
  the author (2026-09-06, *"not really necessary"*). Recorded here so it is not resurrected as a
  styling idea: it would need a second full-screen translucent window, conflicting with
  SHELL-03 / D-12 / D-13.

### Carried from earlier phases (not this phase's work)
- `Cargo.toml` still reads `version = "0.1.0"` while git tags read v0.3.
- Untracked `ssf_*` presets and scratch files await a tracked-vs-ignored decision.
- Overlay single-instance guard — backlog Phase 999.2.
- `apply --early-game` (a `--tiers` override now exists, but not the forced-fallback pair).

</deferred>

<open_questions_for_research>
## For the Researcher

Design is settled; these are mechanism questions.

1. **egui control vocabulary at 460px.** Which widgets best serve a grouped collapsible checkbox
   list, a comparator+value rarity selector, and numeric entry with an enable toggle —
   `CollapsingHeader`, `ComboBox`, `DragValue`, `TextEdit`? Confirm shapes against the installed
   egui 0.36.1 source; Phase 13 found RESEARCH sketches drifted from the real API three times
   (notably `eframe::App::ui`, not `update`).
2. **Scrolling inside the stripe** with an accordion open — `ScrollArea` behaviour, and whether an
   expanded row can be kept in view when it opens near the bottom.
3. **Autosave debouncing.** Is a write-per-keystroke acceptable for a small JSON file, or should
   edits be coalesced? If coalesced, what guarantees the last edit is flushed before Quit?
4. **Serialising `Config` back to `dynamic.json` without losing fidelity** — the store is a full
   `Config`; round-tripping must not drop unknown or defaulted fields, reorder keys destructively,
   or strip the `enabled` field's `skip_serializing_if` behaviour in a way that changes meaning.
   This is the highest-risk item: a lossy write silently damages the user's store.
5. **Testing mutation logic headlessly** — confirm the pure-function shape used in Phase 13
   extends to add/delete/move/edit, and whether `egui_kittest` is now worth adopting for the
   widget layer (Phase 13's research recommended deferring that decision to this phase).

</open_questions_for_research>

---

*Phase: 14-Rule Editor + Apply*
*Context gathered: 2026-09-06*
