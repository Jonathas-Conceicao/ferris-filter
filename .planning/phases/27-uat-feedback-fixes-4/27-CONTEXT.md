# Phase 27: UAT Feedback Fixes 4 - Context

**Gathered:** 2026-10-15
**Status:** Ready for planning
**Mode:** Locked decisions from the author's fourth hands-on UAT round of v0.5 (list approved by the
author); run autonomously, no questions. No UI-SPEC: the decisions below are the spec.

<domain>
## Phase Boundary

Three overlay changes plus a documentation round (requirements UAT4-01..UAT4-04): a right-aligned
Order column in the CONFIG Custom styles table, colour dropdowns that leave room for their scroll
bar, a new **Simple Defences** condition beside the existing **Defences** card, and the author's
UAT results recorded in the earlier phases' manual verification docs (plus two backlog items).

The author dislikes "texty" UI: minimal text, tooltips over captions. Backlog 999.11 covers a later
full UI review; do not redesign beyond this list.

Out of scope: generator output changes (`generate` / `artifact_guard` / `generate_snapshot`
fixtures stay byte-identical), the CLI's behaviour, Ward/`BaseWard` support, any class check on
defence conditions.

</domain>

<decisions>
## Implementation Decisions (all locked)

### UAT4-01: Custom styles Order column right-aligned

- In the CONFIG tab's **Custom** styles table, the Order column (the up/down arrows) is
  right-aligned at the table's right edge, like the reorder arrows on the RULES tab rows
  (`rule_list.rs` draws them in a `Layout::right_to_left` strip).
- The "Order" column header aligns with the arrows (right-aligned over them).
- The Built-in table is unchanged (no Order column); the shared columns (Visibility | Name |
  Preview) stay aligned across both groups.
- Keep or update the existing layout probes/tests in `config_view.rs` (~1905 "Only the custom group
  has an Order column, and its rows line up under it") so they assert the right alignment.

### UAT4-02: colour dropdown scroll bar

- `colour_combo` in `src/overlay/body.rs` (~2690) draws each row as `ui.horizontal` (colour dot +
  `selectable_label`). When the popup scrolls, the scroll bar draws on top of the longest colour name
  instead of sitting at the right edge with room for it. The minimap shape combo (plain
  `selectable_label`s in the same `ComboBox` popup) is correct.
- **Find the real cause** (read egui 0.36.2's `containers/combo_box.rs` / `scroll_area.rs` /
  `menu`/popup layout in `~/.cargo/registry/src/*/egui-0.36.2/`; the popup runs the contents inside
  `ScrollArea::vertical()` with `wrap_mode = Extend` and `set_min_width(available_width)`), and fix
  it so colour rows reserve the same room for the scroll bar as the shape combo. Do not paper over
  it with a magic padding constant unless that is what the cause analysis shows is correct; record
  the cause in the plan SUMMARY.
- Applies to every colour dropdown: minimap colour, beam (PlayEffect) colour, and the CONFIG style
  editor's colour pickers (they all go through `colour_combo` or must be made to).
- Add a headless test if feasible (e.g. the row's/label's rect stays within the scroll area's inner
  width minus the scroll bar width, matching the shape combo's behaviour); otherwise a focused unit
  test of whatever helper the fix introduces, plus a manual-verification line.

### UAT4-03: Simple Defences (new condition), alongside Defences

- **Keep** the existing **Defences** card (per-type toggle + operator + slider + Between) unchanged
  as the advanced card.
- **Add Simple Defences**: one card per rule with three rows **AR / EV / ES** (Armour, Evasion,
  Energy Shield), each a compact segmented control **Any | Has | No**:
  - **Any** = no line for that type.
  - **Has** writes `BaseArmour > 0` (resp. `BaseEvasion > 0`, `BaseEnergyShield > 0`).
  - **No** writes `BaseEvasion 0` (bare `0`, the same form `generate` emits for `eq: 0`).
- Below the rows, a display-only strip of the 7 combinations, in this order: **AR, EV, ES, AR+ES,
  AR+EV, EV+ES, AR+EV+ES**, each chip lit or dimmed for whether the current settings match it. A
  combination matches when it contains every Has type and none of the No types (Any is free).
  Not clickable. A tooltip on the strip/chips is fine; no captions.
  - Example: EV: No only -> AR, ES, AR+ES lit; the rest dimmed.
  - All Any -> all 7 lit. A setting that matches nothing (e.g. all No) -> all dimmed (no warning).
  - Implement the matching as a small pure function and test all 27 settings.
- **Picker:** both **Defences** and **Simple Defences** sit right after Rarity in the + condition
  picker (Item section). They are **mutually exclusive**: a rule has at most one of them. When
  either is present (lines or a pending card), both entries are greyed in the picker, and clicking
  either focuses the existing card (reuse the Phase 26 duplicate-greying mechanism:
  `conditions::presence`, `ConditionEntry`, `FocusTarget`, `list.focus_card`).
- Adding Simple Defences with every row on Any writes nothing (a pending card, like an empty
  Defences card). Removing the card removes all its lines.
- **Loading / classification:** if every `BaseArmour` / `BaseEvasion` / `BaseEnergyShield` line in
  the rule is "zero based" and there is at most one line per type, the rule shows a Simple Defences
  card. Zero based means: value `0` with no operator, `=` or `==` (-> **No**), or `> 0`, or `>= 1`
  (-> **Has**). Anything else (other values, other operators such as `>= 0` / `!= 0` / `< 5`, two
  lines for one type, a Between pair) -> the whole set loads as the existing Defences card.
- **Byte preservation:** loaded lines keep their exact bytes until the user changes that type's
  segment. Setting Any removes that type's line; switching Has <-> No rewrites only that line (to
  the canonical `X > 0` / `X 0`); re-selecting the current segment changes nothing. Untouched types
  are never normalized (PARSE-04).
- **Card stability (Claude's decision):** the card kind must not flip under the user while they edit.
  Classification is from the rule's lines, but once a rule shows a Defences card (loaded or added
  from the picker), edits inside that card never turn it into a Simple Defences card mid-session
  (e.g. picking `> 0` on a slider); likewise a Simple Defences card always produces zero-based lines
  so it stays Simple. Mechanism is the planner's choice (e.g. a per-rule "advanced defences" marker
  in the rule list's editor state, cleared on reload/revert or when the card is removed). After a
  reload, classification is from the file again.
- **No class check:** a rule without armour classes may still use either card. Do not block or warn
  (backlog item added under UAT4-04).
- UI names: "Defences" and "Simple Defences" (existing spelling). Row labels AR / EV / ES with
  tooltips naming Armour / Evasion / Energy Shield.
- **Tests:** loading classification (zero-based vs custom vs mixed, duplicates, Between pair,
  `== 0` / `= 0` / bare `0` / `> 0` / `>= 1`), byte preservation of untouched and loaded lines,
  writes for Has / No / Any, card removal removes all lines, picker exclusivity (both greyed,
  focus the existing card either way), combination strip logic for all 27 settings, card stability.

### UAT4-04: record UAT results; backlog

- Fill **Result** as `pass` for every item in
  `.planning/phases/24-uat-feedback-fixes/24-MANUAL-VERIFICATION.md`,
  `.planning/phases/25-uat-feedback-fixes-2/25-MANUAL-VERIFICATION.md` and
  `.planning/phases/26-uat-feedback-fixes-3/26-MANUAL-VERIFICATION.md` (the author reported
  conditions, BaseType list, Yes/No, style rename, Between, greying, Quit confirm, Defences all
  working; earlier rounds were exercised through feedback). Exception: in 26, **UAT3-05** Defences =
  `pass (rework: Simple Defences added in Phase 27)`.
- `.planning/phases/23-auto-reload-filter-in-game-on-apply/23-MANUAL-VERIFICATION.md` item 7
  (chat already open with text): `accepted: reload not sent, no false "Reloaded" claim shown; left as is`.
- **Debug beside release** (item 7 of
  `.planning/phases/22-single-instance-overlay/22-MANUAL-VERIFICATION.md`; the author named it with
  the Phase 23 docs): `pass`.
- Phase 16 keyword-table sign-off (`.planning/phases/16-lossless-filter-parser/16-HUMAN-UAT.md`,
  test 1): `partial pass: untested keywords deferred to backlog` (update its summary counts and
  status consistently).
- Requirements of phases 24-26 may be marked UAT-verified in REQUIREMENTS.md traceability if that
  table distinguishes it; do not mark phases complete in the roadmap beyond what the docs support.
- Add two backlog items in `.planning/ROADMAP.md` (next free 999.x numbers, currently 999.12 and
  999.13, same format as 999.10/999.11: heading, **Goal:**, **Requirements:** TBD, **Plans:** TBD,
  `Plans:` list with the TBD line):
  - "Keyword table: verify remaining keywords in game"
  - "Guide rules away from conditions that cannot match (e.g. defences on rules without armour
    classes)"

### General

- Edits touch only the edited lines; never normalize untouched operators (PARSE-04).
- Minimal text; tooltips over captions.
- Tests never write to the real game folder or the repo's committed styles.json/tiers.json/
  config.json (use temp roots); never send real keystrokes (fake sinks); never launch or kill `ff`.
- Build/test with `CARGO_TARGET_DIR=C:/Users/jonat/AppData/Local/Temp/ff-target`; full
  `cargo test` passes and `cargo fmt --check` is clean.
- The author has uncommitted local edits to `styles.json` and `tiers.json`: never commit, revert
  or overwrite them. With those local edits in place ~15 tests that read the repo's `styles.json`
  fail (pre-existing); the orchestrator runs the full suite with them stashed. Executors run
  targeted tests (`cargo test --lib overlay::` etc.) and must not stash/commit those two files.
- Docs: CLAUDE.md overlay notes (Defences + Simple Defences, Order column) and CHANGELOG
  `[Unreleased]`.

### Claude's Discretion

- Plan split and waves; the colour-combo fix once the cause is known; segmented-control widget
  (reuse the Phase 24/25 Yes | No control style if there is one); chip look (lit = accent fill,
  dimmed = weak text); the card-stability mechanism.

</decisions>

<code_context>
## Existing Code Insights

- `src/overlay/config_view.rs`: styles table per group, `COL_ORDER`, header row (~603, "Order"
  ~623), row arrows via `widgets::arrow_button` (~771-845), alignment tests (~1905, ~2201).
- `src/overlay/rule_list.rs`: RULES row arrows drawn right-to-left (~380-423); `FocusTarget` (~64),
  `focus_card`.
- `src/overlay/body.rs`: `colour_combo` (~2690), minimap shape combo (~2756), effect colour,
  condition picker entries and greying (`pick_condition`, ~288-330), Defences card (~472-640),
  Between (~638-900), Yes | No boolean control, tests (Defences ~3612, Between ~4710).
- `src/overlay/conditions.rs`: `Part`, `CardPlan` (incl. `CardPlan::Defences`), `card_plan` (~89),
  `presence` (~140), numeric op choices, Between switch.
- `src/overlay/grammar.rs`: `ConditionEntry` (~32), `CONDITION_GROUPS`, `slider_range`.
- `src/overlay/config_view.rs` style editor colour pickers (check they use `colour_combo`).
- egui 0.36.2 sources: `~/.cargo/registry/src/*/egui-0.36.2/src/containers/combo_box.rs` (~395:
  `Popup::menu(...).width(button width)`, `ScrollArea::vertical()`, `wrap_mode = Extend`).

</code_context>

<specifics>
## Specific Ideas

- `27-MANUAL-VERIFICATION.md`: numbered `**UAT4-NN** - action - Expected: ... - Result: ____`,
  same format as 26's, covering UAT4-01..03 (UAT4-04 is documentation).

</specifics>

<deferred>
## Deferred Ideas

- Ward / `BaseWard` support (valid in PoE2) as a fourth defence type.
- Guidance for conditions that cannot match (backlog, added here).
- Verifying the remaining keywords in game (backlog, added here).
- Full UI review (backlog 999.11).

</deferred>
