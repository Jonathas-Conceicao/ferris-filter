# Phase 35: Prototype Retirement & Docs - Context

**Gathered:** 2026-10-17
**Status:** Ready for planning

<domain>
## Phase Boundary

Remove `ff-proto` and the old `overlay::theme` stripe styling so the skin is the only overlay look
(CLEAN-01), and bring README, CLAUDE.md and CHANGELOG up to what v0.6 ships, with the Cargo
version at `0.6.0` (CLEAN-02). `ff-art` and the committed game art in `assets/` stay.
Runs after quick task 261017 (economy names/styles, staged economy rows, tab `*`, text field gap).
</domain>

<decisions>
## Implementation Decisions

### Removal (CLEAN-01)
- **D-01:** Delete `src/bin/ff-proto.rs`, its `[[bin]]` entry in `Cargo.toml`, and every mention
  (README section, CLAUDE.md row, skin/shell doc comments, ASSETS.md "and `ff-proto`").
- **D-02:** Delete `src/overlay/theme.rs`. The constants still read through it (`LABEL_TAN`,
  `VALUE_TEXT`, `ROW_SUMMARY_SIZE`, `WARM_ACCENT_TEXT`, `SECTION_CREAM`, `ROW_BASE`, ...) are
  replaced at their call sites by the skin palette / skin constants; values that are not in the
  palette yet move into `src/skin/` (no visual change). `poe2_visuals`, `install_fonts`,
  `section_header`, `row_label`, `row_summary` go if unused, or move into the skin if still used.
- **D-03:** Remove the Cinzel font: `assets/Cinzel-Regular.ttf`, `assets/Cinzel-OFL.txt`, the
  `skin::fonts::CINZEL` family registration, the ASSETS.md row and regeneration section.
- **D-04:** The `ff` debug screenshot mode stays.
- **D-05:** Build, full test suite, fmt stay green; clippy no new warnings (baseline 126);
  `tests/fixtures/seed-universal.filter` and `generate` output unchanged.

### Docs (CLAUDE-02)
- **D-06:** README: a general page with current screenshots and a proper show of the repo (what
  FerrisFilter is, the overlay and its tabs, hotkey, CLI commands, building, `ff-art`, credits /
  GGG disclaimer). The user will fully rewrite it by hand later, so keep it plain and accurate,
  not polished. Screenshots rendered with `ff --screenshot` from `tests/ui/test_ff.filter` in a
  temp home, committed under `docs/screenshots/` (a few: Rules, a rule expanded, Config style
  editor, Economy, About). Drop the personal `C:\Users\jonat\...` path (use `%USERPROFILE%\...`).
- **D-07:** CLAUDE.md: correct, not rewrite. Remove `ff-proto`/theme/Cinzel mentions, make sure
  the overlay notes match what ships. No restructuring of the long overlay notes.
- **D-08:** CHANGELOG `[Unreleased]`: light cleanup only (it will be reset by hand later):
  merge the two `ff-proto` prototype entries into one line saying the prototype was added and then
  removed, add the phase 35 removals, trim obvious duplication. Do not spend much effort.
  The `[Unreleased]` -> `[0.6.0]` move happens at `/gsd-complete-milestone`, not here.
- **D-09:** Bump `Cargo.toml` (and `Cargo.lock`) to `0.6.0` in this phase.

### Claude's Discretion
- Where moved theme constants live inside `src/skin/` and their names.
- Exact screenshot set and README section order.
</decisions>

<code_context>
## Existing Code Insights
- `overlay::theme` users: `src/overlay/body.rs`, `config_view.rs`, `rule_list.rs`, `widgets.rs`
  (44 references); `theme.rs` constants already alias `skin::palette` (`pal::*`) for most values.
- `src/skin/fonts.rs` registers `CINZEL`; `theme::DISPLAY_FONT` is its only user.
- `src/overlay/apply.rs:23` has a doc comment mentioning `install_fonts`.
- Screenshot mode flags: see CLAUDE.md `ff` row; needs `FERRISFILTER_HOME`/`USERPROFILE` under Temp;
  call the built exe directly, not `cargo run`.
</code_context>

<specifics>
## Specific Ideas
- The user prefers a human-written README and CHANGELOG for human readers; this phase only leaves
  them accurate and current.
</specifics>

<deferred>
## Deferred Ideas
- Full human rewrite of README and CHANGELOG (user, after the milestone).
- League picker / price source UI (backlog 999.17).
</deferred>
