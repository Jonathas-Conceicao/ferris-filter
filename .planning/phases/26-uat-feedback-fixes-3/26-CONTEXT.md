# Phase 26: UAT Feedback Fixes 3 - Context

**Gathered:** 2026-10-14
**Status:** Ready for planning
**Mode:** Locked decisions from the author's third hands-on UAT round of v0.5 (list approved by the
author); run autonomously, no questions. No UI-SPEC: the decisions below are the spec.

<domain>
## Phase Boundary

Eight overlay changes (requirements UAT3-01..UAT3-08): a grouped CONFIG styles table, stable style
ids so custom styles can be renamed, a style picker fed by the in-memory style set, a steadier rule
editor (no collapse on first pick, operators always visible, one Defences card, Between, no
duplicate conditions, collapsed condition picker), and a tray Quit that asks before discarding
unapplied edits.

The author dislikes "texty" UI: minimal text, tooltips over captions. Backlog 999.11 covers a later
full UI review; do not redesign beyond this list.

Out of scope: generator output changes (`generate` / `artifact_guard` / `generate_snapshot`
fixtures stay byte-identical), the CLI's behaviour, Ward/`BaseWard` support.

</domain>

<decisions>
## Implementation Decisions (all locked)

### CONFIG -> Styles

**UAT3-01: table per group.** Each group renders as: group heading -> its own column-header row ->
its rows.
- **Built-in**: heading "Built-in", header `Visibility | Name | Preview`, then rows.
- **Custom**: heading "Custom" with **+ ADD STYLE** on the heading line (UAT2-08 placement kept),
  header `Visibility | Name | Preview | Order`, then rows; Order holds the existing up/down arrows
  (disabled at the ends, never above built-ins). The Custom heading and its header show even when
  there are no custom styles (header may be omitted when empty; Claude's discretion).
- Column widths stay aligned across both groups for the shared columns.

**UAT3-02: stable style ids; custom styles renamable.**
- `styles.json` keeps its shape (top-level map name -> style object, file order = display order)
  and every style object gains an `"id"` field, rendered first in the object by
  `loader::render_styles` (one compact line per style, as today).
- Built-in ids are `builtin:<name>` (e.g. `builtin:S`, `builtin:hidden`); built-in =
  `StyleSet::is_builtin` (do not invent a new definition).
- Custom ids: random 6-8 lowercase alphanumerics, generated non-conflicting against every id in
  the set (retry until unique; test it with an injected/forced-collision generator). A custom id
  never starts with `builtin:`. No new heavy dependency for randomness (a small seeded generator
  from time/process entropy or an existing dep is fine; Claude's discretion).
- Loading: duplicate ids, a built-in whose id is not `builtin:<name>`, or a custom id starting with
  `builtin:` are validation errors. A style **without** an id (older files, temp test fixtures,
  the author's working copy) is assigned one at load (built-in: `builtin:<name>`; custom: random).
  Ids assigned at load are persisted by the next successful combined Apply (styles.json joins the
  write set) but are **not** shown as an unapplied CONFIG edit. Claude's discretion how this is
  tracked (e.g. a "needs id persist" flag on ConfigState).
- The `id` field is ignored by the generator/economy: `config.json` rules and the economy emitter
  keep referencing styles **by name**, so `generate` output and the artifact_guard /
  generate_snapshot fixtures stay byte-identical. If a custom style that config.json references is
  renamed, `generate` fails with the existing clear unknown-style error (document in CLAUDE.md).
- **Filter tag format**: `# ff:style <id> <name>`. The id links; the name is informational and is
  refreshed on Apply (after a rename, every tag for that id gets the new name on the next Apply).
- **Resolution** (tag text after `ff:style`, trimmed): if the first whitespace-separated token is a
  known id -> that style. Otherwise, if the whole remainder equals a style name (old
  `# ff:style kite` tags; names may contain spaces, e.g. "Build Gear") -> that style, and the tag
  is rewritten to the new `<id> <name>` form on Apply. Otherwise unknown (existing unknown-style
  handling: report + drop tag on Apply, as Phase 25 does). Keep this fallback; it is cheap.
- Tag rewrites touch only the `# ff:style` line of the affected block (PARSE-04); a tag already in
  the right `<id> <name>` form is not rewritten. Applying a style from the rule editor writes the
  new form.
- **Rename in CONFIG**: custom styles get an editable name (inline text field in the Name column;
  commit on Enter/focus-out, draft-buffered so typing does not jump); name must be unique and
  non-empty using the same rules as `StyleSet::insert_new` (no padding/control chars, no
  case-only shadow of a built-in); an invalid name shows a tooltip/red outline and is not
  committed. Built-ins are not renamable. A rename is an in-memory CONFIG edit (dirty,
  Revert-able), persisted on the combined Apply. Since `StyleSet` is keyed by name, rename must
  keep the style's position in file order.
- Everything that today keys styles by name inside the overlay (rule tag status, style counts,
  visibility toggle, CONFIG edits) must resolve through the id where a tag is involved so a rename
  does not orphan rules.
- **Committed styles.json**: the orchestrator (not the executors) adds the ids to the committed
  `styles.json` after execution, without disturbing the author's uncommitted local edits to that
  file. Executors must **not** edit or commit the repo-root `styles.json`, and tests must not
  depend on the repo-root styles.json having ids (use in-memory/temp fixtures; tests that
  `include_str!` styles.json must work with or without ids).

**UAT3-03: style picker lists the in-memory style set.** The rule editor's style dropdown lists the
CONFIG tab's in-memory (unapplied) StyleSet, including added and renamed styles not yet applied, in
display order. Choosing one writes its tag + actions from that in-memory style (consistent with the
all-or-nothing combined Apply of Phase 25, which renders with the pending styles). The rule list's
tag-status (known/unknown) judges against the same in-memory set.

### Rule editor

**UAT3-04: no auto-collapse; operator always visible.**
- Clicking a value in a multi-select (Class, Rarity, BaseType list, etc.) never collapses the rule
  row/card. Find and fix the **root cause** of the collapse on the first Class click (suspected: the
  row's identity/key or height changes when a pending condition becomes a real line, so the
  expansion state keyed by the old id is lost, or the click falls through to the row header).
  Add a regression test (headless egui harness used by existing body/rule_list tests).
- Every condition (Class, Rarity, numeric, boolean-with-op, etc.) shows its operator dropdown as
  soon as it is added, before any value is picked. The chosen operator is remembered on the
  pending row and used when the first value creates the line.
- A condition with no value yet is still not written to the file (pending row, Phase 24 pattern).

**UAT3-05: one "Defences" condition.**
- The condition picker's **Item** section gets one entry **Defences** right after Rarity. The
  separate `BaseArmour` / `BaseEvasion` / `BaseEnergyShield` entries and the "Defences" group are
  removed from the picker.
- The Defences card has three toggles - **Armour, Evasion, Energy Shield**. Each toggle, when on,
  shows its own operator dropdown and slider (existing ranges from `grammar::slider_range`) and
  writes its own line (`BaseArmour`, `BaseEvasion`, `BaseEnergyShield`). Turning a toggle off
  removes that line only. Adding Defences with nothing toggled writes nothing (pending).
- Existing rule lines for those three keywords display inside the Defences card (one card per
  rule), with their original bytes until edited.
- Remove `BaseDefencePercentile` from the keyword table (`src/filter/keywords.rs`) and grammar
  (same treatment as Phase 24/25 removals: a foreign line still loads losslessly as an
  unknown-keyword raw row, parse warning not error, does not block Apply; add a test like
  `gate_ignores_removed_poe1_keyword` / `removed_poe1_keyword_is_a_raw_row`; update
  `documented_vocabulary_present` and counts).
- Ward/`BaseWard` stays dropped. **Future work:** Ward is valid in PoE2 but not supported yet; it
  would become a fourth Defences toggle.

**UAT3-06: no duplicate conditions; Between.**
- In the + condition picker, a condition already on the rule is greyed out (still clickable);
  clicking it closes the picker and focuses/scrolls to the existing card (`scroll_to_me` + a brief
  highlight or keyboard focus; Claude's discretion). Defences counts as present if any of its
  three lines (or a pending Defences card) exists; its missing toggles are added from the card.
- Numeric conditions get a **Between** operator: one card with two values (min, max) writing two
  lines `X >= min` and `X <= max`. Switching to Between from a single-line operator keeps the
  existing line as one bound (edited minimally) and adds the other line right after it; switching
  away from Between removes the second line. min > max is prevented or flagged (Claude's
  discretion; never write an impossible pair silently).
- On load, a rule that has **exactly** one lower-bound line (`>=` or `>`) and one upper-bound line
  (`<=` or `<`) for the same numeric keyword (and no other line of that keyword) displays as one
  Between card; each line keeps its original operator and bytes until that bound is edited
  (editing the min touches only the min line). Other duplicates in a loaded filter (two `>=`, a
  `==` plus a `>=`, three lines, ...) still show as separate cards - no merging, no normalization.
- Each defence inside the Defences card can also use Between.

**UAT3-07: condition picker opens with every section collapsed** (today `default_open(gi == 0)`
opens "Item"; body.rs ~445). Search/filter behaviour inside the picker, if any, may still force
matching sections open.

### App

**UAT3-08: Quit with unapplied changes asks first.**
- Tray Quit while either tab has unapplied edits (the combined Phase 25 dirty state: RULES edits,
  CONFIG edits, or a staged economy op) shows the stripe (as tray Show does) with a modal confirm:
  **Apply & quit**, **Quit without applying**, **Cancel**.
  - Apply & quit runs the combined Apply; quits only if it succeeds. On refusal/failure (changed on
    disk, grammar error, etc.) the stripe stays open showing the refusal (with OVERWRITE/RELOAD as
    today); the confirm closes.
  - Quit without applying quits as today.
  - Cancel closes the confirm; nothing changes.
- No pending edits -> quit immediately as today.
- Respect the Phase 22 shutdown ordering exactly (hotkey unregistered, tray dropped, instance lock
  released before the viewport close; see app.rs ~1623-1680 and ~2140). The confirm only decides
  *whether* that sequence runs. A second tray Quit while the confirm is open just keeps it open.
- Keep the decision logic in a small pure function (dirty state x choice -> action) so it is
  unit-testable without a window.

### General
- Edits touch only the edited lines; never normalize untouched operators (PARSE-04).
- Minimal text; tooltips over captions.
- Tests never write to the real game folder or the repo's committed styles.json/tiers.json/
  config.json (use temp roots); never send real keystrokes (fake sinks); never launch or kill `ff`.
- Build/test with `CARGO_TARGET_DIR=C:/Users/jonat/AppData/Local/Temp/ff-target`; full
  `cargo test` passes and `cargo fmt --check` is clean.

### Claude's Discretion
- Plan split and waves; id generator implementation; how load-assigned ids are tracked for
  persistence; rename widget details; focus/highlight effect for an existing condition; how
  Between handles min > max; Quit-confirm widget layout (stripe modal, short labels).

</decisions>

<code_context>
## Existing Code Insights

- `src/models/config.rs`: `Style` (serde `deny_unknown_fields`, needs `id`), `StyleSet(IndexMap<
  String, Style>)` with `REQUIRED`, `is_builtin`, `insert_new`, `remove_custom`, `move_custom`,
  `committed_for_tests` (`include_str!` of repo styles.json), `COMMITTED_ORDER` test (~1752).
- `src/loader.rs`: `load_styles`, `render_styles` (~199), `render_tiers`.
- `src/styled.rs`: `rewrite_styled`, `drop_unknown_style_tags`, `style_name_ok`, `StyleReport`.
- `src/filter/meta.rs` / `src/filter/edit.rs`: `# ff:` meta-line reading/writing (`ff:style`).
- `src/overlay/rules.rs`: tag status (~85, ~198), per-style counts (~598).
- `src/overlay/config_state.rs` / `config_view.rs`: style table (Phase 25 header row, arrows,
  Add style on the Custom heading), visibility switch, Revert; `apply_all.rs`: combined Apply.
- `src/overlay/body.rs`: condition picker (`CollapsingHeader` with `default_open(gi == 0)` ~445),
  pending rows, condition cards, operator dropdowns, style section (~1730).
- `src/overlay/grammar.rs`: `CONDITION_GROUPS` (Item, Defences, Sockets, Status, Mods, Endgame,
  Other), `slider_range`, display names; `src/filter/keywords.rs`: `KEYWORDS`
  (`BaseDefencePercentile` ~100), removed-keyword tests (~224).
- `src/overlay/rule_list.rs`: row expansion state, up/down arrows (reused by the styles table).
- `src/overlay/app.rs`: tray command polling and the Phase 22 Quit sequence (~1623, ~2140);
  `src/overlay/tray.rs`: `OverlayCommand::Quit`.

</code_context>

<specifics>
## Specific Ideas

- Tests: id generation unique under forced collisions; built-in ids fixed; missing ids assigned at
  load and persisted on next Apply without a dirty indicator; duplicate/invalid ids rejected;
  `render_styles` writes `id` first; tag `<id> <name>` resolves by id; old name tag resolves and is
  rewritten on Apply; rename of a custom style keeps rules linked and refreshes the tag name on
  Apply; built-in rename refused; generate output byte-identical; style picker shows an added but
  unapplied style; first Class click keeps the row expanded; operator shown on a pending row and
  used when the first value lands; Defences card writes/removes individual lines and shows loaded
  lines; BaseDefencePercentile raw row + Apply not blocked; picker greys present conditions;
  Between writes two lines, loads a `>`/`<=` pair as one card without rewriting bytes, editing min
  touches only the min line; other duplicates stay separate; picker sections collapsed by default;
  Quit decision function for dirty/clean x Apply&quit success/failure / quit / cancel.
- Docs: CLAUDE.md (styles.json schema with ids, `# ff:style <id> <name>` format and name fallback,
  rename + generate caveat, Defences/Between/Quit overlay notes) and CHANGELOG `[Unreleased]`;
  `26-MANUAL-VERIFICATION.md` (one line per item).

</specifics>

<deferred>
## Deferred Ideas

- Ward / `BaseWard` support (valid in PoE2) as a fourth Defences toggle.
- Full UI review (backlog 999.11).

</deferred>
