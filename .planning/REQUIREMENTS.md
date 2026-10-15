# Requirements: FerrisFilter — v0.5 Generic Filter Editor & Initial App Configuration

**Defined:** 2026-10-12
**Core Value:** `generate` turns the config into a correct, deterministic PoE2 loot filter.

**Milestone goal:** FerrisFilter can open, edit and save any working PoE2 filter, not just its own
output, with the style palette and economy tier cutoffs configurable from the overlay.

**Acceptance bars (author-set):**

- The editor supports the **full** filter grammar documented at
  https://www.pathofexile.com/item-filter/about. There is no "unsupported condition" class of
  block.

- `config.json → universal.filter` is unaffected: `artifact_guard` and `generate_snapshot` pass
  without re-baselining the fixture.

## v0.5 Requirements

### Filter Parsing (PARSE)

- [x] **PARSE-01

**: User can open any working PoE2 filter without error: FilterBlade/NeverSink,
  poe2filter.com, hand-written, or FerrisFilter output.

- [x] **PARSE-02

**: Every block type, condition, operator and action documented at
  pathofexile.com/item-filter/about is recognized, including `Continue`, `Import` and `Minimal`.

- [x] **PARSE-03

**: Saving a filter with no edits writes a byte-identical file. Line endings, BOM,
  indentation, comments and spacing are all preserved. This is verified against the real
  FilterBlade/NeverSink filter.

- [x] **PARSE-04

**: Saving after editing one block changes only that block's lines. Every other byte
  of the file is untouched.

### Rule Editor (EDIT)

- [x] **EDIT-01**: User can view and edit every condition on any block: all operators, and
  multi-value lists. No condition is read-only.

- [x] **EDIT-02**: User can view and edit every action on any block: colours, font size, alert
  sounds (built-in and custom), minimap icon, beam effect, and drop-sound toggles.

- [x] **EDIT-03**: User can switch a block between `Show`, `Hide` and `Minimal`, and toggle
  `Continue`.

- [x] **EDIT-04**: Disabling a rule comments its block out. Commented-out blocks in a loaded filter,
  such as NeverSink's, appear as disabled rules and can be enabled.

- [x] **EDIT-05**: User can add, delete, duplicate and reorder rules.
- [x] **EDIT-06**: User can apply a named style to a rule, which sets the rule's style actions.
- [x] **EDIT-07**: User can search the rule list, and the editor stays responsive on a filter of
  about 460 blocks (the NeverSink sample).

- [x] **EDIT-08**: Apply writes the edited filter, and Revert discards unsaved edits. This is the
  v0.4 model: edits are held in memory and Apply is the single writer.

- [x] **EDIT-09**: Apply refuses to overwrite a filter that changed on disk after it was loaded,
  and tells the user why.

- [x] **EDIT-10**: The first time FerrisFilter writes to a filter, it keeps a backup copy of the
  original.

### Economy Section (ECON)

- [x] **ECON-01**: Economy-priced blocks are written after the general rules, inside a section
  delimited by comment markers.

- [x] **ECON-02**: The economy section is not shown in the rule editor. The rule list shows a
  summary instead, e.g. "+ 42 locked rule blocks".

- [x] **ECON-03**: Apply regenerates the economy section from `prices.json`, `tiers.json` and the
  styles, changing no byte outside the markers.

- [x] **ECON-04**: User can add an economy section to a filter that has none (a new empty filter or
  a foreign filter), and can remove it.

### Filter Location (STORE)

- [x] **STORE-01**: By default the overlay edits the game-folder `dynamic.filter` directly. There
  is no intermediate JSON store.

- [x] **STORE-02**: User can select a different filter in the game folder from the configuration
  tab, and the choice is remembered across launches.

- [x] **STORE-03**: A new filter starts empty, with an optional economy section.
- [x] **STORE-04**: `dynamic.json`, `dynamic_base.json`, the overlay's JSON store, and CLI
  `apply --reset/--base/--store` are removed. CLI `apply` operates on the filter file. The breaking
  change is noted in `CHANGELOG.md`.

- [x] **STORE-05**: `ff` can be launched from anywhere, including by double-clicking the exe. It
  finds the repo files (`styles.json`, `tiers.json`, `prices.json`, …) without relying on the
  process working directory.

### Styles (STYLE)

- [x] **STYLE-01**: Named styles live in a committed repo-root `styles.json`. Adding a new style
  requires no code change.

- [x] **STYLE-02**: A rule naming an unknown style, or an invalid `styles.json`, produces a clear
  error at load time.

- [x] **STYLE-03**: A rule that uses a named style remembers it (via a comment tag in the filter).
  When the style changes, the rule's actions update on the next Apply.

- [x] **STYLE-04**: `generate` output is unchanged: the `seed-universal.filter` fixture stays
  byte-identical.

### Configuration Tab (CONF)

- [x] **CONF-01**: The overlay has a configuration tab, separate from the rule editor.
- [x] **CONF-02**: User can add, edit and delete named styles: text, border and background colour
  (colour picker), font size, sound, minimap icon and beam. Built-in styles cannot be deleted.

- [x] **CONF-03**: User can edit the economy tier cutoffs in `tiers.json` (not prices). Invalid
  cutoffs are shown inline and cannot be applied.

- [x] **CONF-04**: The configuration tab has its own Apply and Revert, and Apply writes all of its
  files or none of them.

### Single Instance (INST)

- [x] **INST-01**: Launching `ff` while it is already running brings up the running stripe (even
  when hidden to the tray) and exits, instead of opening a second instance.

### In-Game Reload (RELOAD)

- [x] **RELOAD-01**: After a RULES-tab Apply that writes the filter, the overlay sends
  `/reloaditemfilter` to a running Path of Exile 2 once (one click, one command), controlled by a
  CONFIG setting "Reload filter in game after Apply" (persisted, default on); when nothing is sent
  (game not running, setting off, send failed) the Refresh reminder is shown with the reason.

### UAT Feedback (UAT)

- [x] **UAT-01**: A RULES Apply whose in-game reload was skipped because the game is not running shows no reload message; other skip reasons keep their message.
- [x] **UAT-02**: CONFIG → Styles lists built-in styles first and custom styles second under separate headings with a divider (no "built-in" badge); only custom styles can be removed.
- [x] **UAT-03**: Each style in CONFIG → Styles has an on/off slide switch (on = Show, off = Hide) that takes effect on the next RULES Apply via the style rewrite and economy regeneration.
- [x] **UAT-04**: The style config and the per-rule action control offer only Show and Hide; a foreign Minimal block is still shown as Minimal and left untouched.
- [x] **UAT-05**: The ✕ that removes a condition is right-aligned so the ✕ buttons line up across rows and rules.
- [x] **UAT-06**: BaseType and other quoted lists are a vertical list (one entry per line with Remove, Enter/+ ADD field, quoted-paste splitting) inside a scrollable panel capped at about 6–8 rows.
- [x] **UAT-07**: The Class condition shows only the grouped class picker (groups from config.json), without the chip list of selected classes.
- [x] **UAT-08**: A styled rule shows no caption text explaining where its style comes from.
- [x] **UAT-09**: PoE1-only conditions are removed from the keyword table and condition picker; a filter line using one still loads losslessly as a raw row and does not block Apply.
- [x] **UAT-10**: Every numeric condition/action default lies inside its slider range (GemLevel fixed), asserted by a table-wide test.
- [x] **UAT-11**: The style section sits at the end of the rule body: a picker of Custom plus every preset with a Show/Hide control; Custom shows and edits the action rows, a preset hides them and tags the rule; preset→Custom keeps the actions and drops the tag.
- [x] **UAT-12**: Boolean conditions are a compact Yes/No control (Yes = True, No = False) with no operator dropdown or toggle; untouched lines keep their operator and text.
- [x] **UAT-13**: Collapsed rules show a short condition summary under the header, hidden when expanded, with correct row virtualization and performance at ~460 blocks.

### UAT Feedback Round 2 (UAT2)

- [x] **UAT2-01**: "Continue to later rules" is its own line after the style section, last in the rule body.
- [x] **UAT2-02**: In the Custom style, the button that adds action lines is labelled "+ style" instead of "+ action".
- [x] **UAT2-03**: Show/Hide sits below the style dropdown and is shown only when Custom is selected; with a preset the style decides Show/Hide and the control is hidden.
- [x] **UAT2-04**: A rule name is a plain text box with no on/off toggle and no derived default name: empty = no `# ff:name` line (clearing removes it), typing adds/updates it.
- [x] **UAT2-05**: Sockets is a plain numeric condition with a 0-6 slider; the PoE1 socket-colour "+ socket" entry is gone.
- [x] **UAT2-06**: `Exceptional` is removed from the keyword table; a foreign line using it loads losslessly as a raw row and does not block Apply.
- [x] **UAT2-07**: The CONFIG panel has no "Config: no unapplied changes" status line and the Styles section has no "new" tag.
- [x] **UAT2-08**: The Add style button sits next to the Custom group heading.
- [x] **UAT2-09**: The styles table has a Visibility | Name | Preview header row; custom styles reorder with up/down arrows (order = styles.json order); built-ins can toggle visibility and edit actions but cannot be renamed, moved or deleted, and stay first.
- [x] **UAT2-10**: APPLY on either tab saves and applies everything pending (styles.json, tiers.json, the filter rendered with the new styles/tiers, then the in-game reload), all-or-nothing: any refusal writes and reloads nothing; OVERWRITE overwrites every changed file, RELOAD reloads from disk; Revert stays per tab; both tabs' stale indicators reflect the single Apply.

### UAT Feedback Round 3 (UAT3)

- [x] **UAT3-01**: The CONFIG styles table is laid out per group as heading -> column header -> rows: "Built-in" (Visibility | Name | Preview) and "Custom" (+ ADD STYLE on the heading; Visibility | Name | Preview | Order, Order holding the up/down arrows).
- [x] **UAT3-02**: Every style in styles.json has a stable `id` (`builtin:<name>` for built-ins; a unique random short lowercase-alphanumeric id for custom styles); rules link to styles by id via `# ff:style <id> <name>` (name informational, refreshed on Apply); old `# ff:style <name>` tags resolve by name and are rewritten on Apply; custom styles are renamable (unique, non-empty), built-ins are not; config.json and `generate` keep referencing styles by name and their fixtures stay byte-identical.
- [x] **UAT3-03**: The rule editor's style picker lists the in-memory (unapplied) style set, including added and renamed styles not yet applied.
- [x] **UAT3-04**: Picking a value in a multi-select condition never collapses the rule row; every condition shows its operator dropdown as soon as it is added; a condition with no value writes no line.
- [x] **UAT3-05**: One "Defences" condition (Item section, after Rarity) with Armour / Evasion / Energy Shield toggles, each with its own operator and slider writing its own `BaseArmour`/`BaseEvasion`/`BaseEnergyShield` line; the separate picker entries and `BaseDefencePercentile` are removed; foreign lines with removed keywords load losslessly as raw rows and do not block Apply.
- [x] **UAT3-06**: The + condition picker greys out conditions already on the rule (clicking focuses the existing card); numeric conditions offer a Between operator writing `>= min` and `<= max` lines; a loaded rule with exactly one lower and one upper bound line for a keyword shows one Between card, bytes preserved until edited.
- [x] **UAT3-07**: The condition picker opens with every section collapsed.
- [x] **UAT3-08**: Tray Quit with unapplied edits on either tab shows the stripe with Apply & quit / Quit without applying / Cancel; Apply & quit quits only when the combined Apply succeeds; with nothing pending Quit exits immediately; the Phase 22 shutdown ordering is kept.

### UAT Feedback Round 4 (UAT4)

- [x] **UAT4-01**: The CONFIG Custom styles table's Order column is right-aligned, its header aligned with the up/down arrows, like the RULES tab's reorder arrows.
- [x] **UAT4-02**: Every colour dropdown (minimap colour, beam colour, CONFIG style editor) leaves room for its scroll bar like the minimap shape dropdown; the scroll bar never draws over a colour name.
- [x] **UAT4-03**: A "Simple Defences" condition (one card per rule; AR / EV / ES rows each Any | Has | No; Has writes `X > 0`, No writes `X 0`, Any writes nothing) sits beside the existing "Defences" card, right after Rarity in the picker; a display-only strip of the 7 defence combinations is lit for those the settings match; the two cards are mutually exclusive (both greyed in the picker when either is present, clicking focuses the existing card); a rule whose defence lines are all zero-based (at most one per type) loads as Simple Defences with bytes preserved until a type is changed, otherwise as Defences; no armour-class check.
- [x] **UAT4-04**: The author's UAT results are recorded in the Phase 16, 23, 24, 25 and 26 manual verification docs, and backlog items are added for verifying the remaining keywords in game and for guiding rules away from conditions that cannot match.

### Default Config Files (DEF)

- [x] **DEF-01**: `styles-default.json` and `tiers-default.json` are committed at the repo root with exactly the HEAD content of `styles.json` / `tiers.json`; nothing in the app ever writes them.
- [x] **DEF-02**: `/styles.json` and `/tiers.json` are git-ignored and untracked; the author's local copies stay on disk byte-for-byte as the live config.
- [x] **DEF-03**: Wherever `styles.json` / `tiers.json` is read (overlay load + watcher, CONFIG tab, `apply`, `generate`, `sync`), an absent live file means the matching `-default.json` in the same directory is read instead; no launch or command creates the live file; the first overlay Apply with CONFIG edits writes it (a missing file does not trip the changed-on-disk guard); `--styles` / `--tiers` stay explicit paths with no fallback; neither file present is an error naming both; the watcher switches source when the live file is created or deleted.
- [x] **DEF-04**: The app-root resolver's marker is `styles-default.json` (FERRISFILTER_HOME, then walk up from the exe, then the working directory).
- [x] **DEF-05**: Every test that read the repo-root `styles.json` / `tiers.json` uses the `-default.json` files, so the suite passes with any local config present; `tests/fixtures/seed-universal.filter` stays byte-identical; CLAUDE.md and CHANGELOG.md describe the split (deleting a live file resets it to the defaults).

## Future Requirements

Deferred. Tracked but not in the v0.5 roadmap.

- **Per-user `styles.json`** in a user config location rather than the repo root (installation
  story).

- **In-game load message** (backlog 999.3): online filters only; needs account publishing.
- **Overlay economy change summary** (backlog 999.4).
- **Locked base filter / `Import` layering** (backlog 999.7).
- **Starting-point presets** (backlog 999.9).
- **Per-block economy markers**, letting economy blocks be interleaved with general rules.
- **Bulk restyle by NeverSink `$type`/`$tier` tags.**

## Out of Scope

| Feature | Reason |
|---------|--------|
| Pretty-printing or normalizing a filter on save | Breaks the byte-identical round trip; only edited lines may change |
| Validating BaseType/Class names against a bundled list | PoE2 itself warns on unknown names at load, and it's a more current source than any list we could ship |
| Full regeneration of the dynamic filter from a model | Intentionally given up: the filter file is the source of truth (999.5) |
| Automatic economy injection into a foreign filter | Economy sections are only added by an explicit user action (ECON-04) |
| Load message / online filter publishing | Online-only and outward-facing; spike findings in backlog 999.3 |

## Decisions Recorded at Definition

- **Rules remember their style via a comment tag** (STYLE-03), so editing a style in the config
  tab updates every rule using it on the next Apply. Chosen by Claude on the author's delegation
  (2026-10-12), to be revisited after hands-on use.

- **Economy blocks sit after the general rules** in one fenced, locked section, hidden from the
  editor (author, 2026-10-12).

- **Commenting out is disabling**, for our own rules and for foreign commented-out blocks alike
  (author, 2026-10-12).

- **Obsolete pieces are removed, not deprecated** (author, 2026-10-12): the project is unreleased.

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| PARSE-01 | Phase 16 | Pending |
| PARSE-02 | Phase 16 | Pending |
| PARSE-03 | Phase 16 | Pending |
| PARSE-04 | Phase 16 | Pending |
| EDIT-01 | Phase 20 | Complete (UAT pending) |
| EDIT-02 | Phase 20 | Complete (UAT pending) |
| EDIT-03 | Phase 19 | Complete |
| EDIT-04 | Phase 19 | Complete |
| EDIT-05 | Phase 19 | Complete |
| EDIT-06 | Phase 20 | Complete (UAT pending) |
| EDIT-07 | Phase 19 | Complete |
| EDIT-08 | Phase 19 | Complete |
| EDIT-09 | Phase 18 | Complete |
| EDIT-10 | Phase 18 | Complete |
| ECON-01 | Phase 18 | Complete |
| ECON-02 | Phase 19 | Complete |
| ECON-03 | Phase 18 | Complete |
| ECON-04 | Phase 18 | Complete |
| STORE-01 | Phase 18 | Complete |
| STORE-02 | Phase 21 | Complete (UAT pending) |
| STORE-03 | Phase 18 | Complete |
| STORE-04 | Phase 18 | Complete |
| STORE-05 | Phase 18 | Complete |
| STYLE-01 | Phase 17 | Complete |
| STYLE-02 | Phase 17 | Complete |
| STYLE-03 | Phase 20 | Complete (UAT pending) |
| STYLE-04 | Phase 17 | Complete |
| CONF-01 | Phase 21 | Complete (UAT pending) |
| CONF-02 | Phase 21 | Complete (UAT pending) |
| CONF-03 | Phase 21 | Complete (UAT pending) |
| CONF-04 | Phase 21 | Complete (UAT pending) |
| INST-01 | Phase 22 | Complete (UAT pending) |
| RELOAD-01 | Phase 23 | Complete (UAT pending) |
| UAT-01 | Phase 24 | Complete (UAT verified) |
| UAT-02 | Phase 24 | Complete (UAT verified) |
| UAT-03 | Phase 24 | Complete (UAT verified) |
| UAT-04 | Phase 24 | Complete (UAT verified) |
| UAT-05 | Phase 24 | Complete (UAT verified) |
| UAT-06 | Phase 24 | Complete (UAT verified) |
| UAT-07 | Phase 24 | Complete (UAT verified) |
| UAT-08 | Phase 24 | Complete (UAT verified) |
| UAT-09 | Phase 24 | Complete (UAT verified) |
| UAT-10 | Phase 24 | Complete (UAT verified) |
| UAT-11 | Phase 24 | Complete (UAT verified) |
| UAT-12 | Phase 24 | Complete (UAT verified) |
| UAT-13 | Phase 24 | Complete (UAT verified) |
| UAT2-01 | Phase 25 | Complete (UAT verified) |
| UAT2-02 | Phase 25 | Complete (UAT verified) |
| UAT2-03 | Phase 25 | Complete (UAT verified) |
| UAT2-04 | Phase 25 | Complete (UAT verified) |
| UAT2-05 | Phase 25 | Complete (UAT verified) |
| UAT2-06 | Phase 25 | Complete (UAT verified) |
| UAT2-07 | Phase 25 | Complete (UAT verified) |
| UAT2-08 | Phase 25 | Complete (UAT verified) |
| UAT2-09 | Phase 25 | Complete (UAT verified) |
| UAT2-10 | Phase 25 | Complete (UAT verified) |
| UAT3-01 | Phase 26 | Complete (UAT verified) |
| UAT3-02 | Phase 26 | Complete (UAT verified) |
| UAT3-03 | Phase 26 | Complete (UAT verified) |
| UAT3-04 | Phase 26 | Complete (UAT verified) |
| UAT3-05 | Phase 26 | Complete (UAT verified) |
| UAT3-06 | Phase 26 | Complete (UAT verified) |
| UAT3-07 | Phase 26 | Complete (UAT verified) |
| UAT3-08 | Phase 26 | Complete (UAT verified) |
| UAT4-01 | Phase 27 | Complete (UAT pending) |
| UAT4-02 | Phase 27 | Complete (UAT pending) |
| UAT4-03 | Phase 27 | Complete (UAT pending) |
| UAT4-04 | Phase 27 | Complete |
| DEF-01 | Phase 28 | Complete |
| DEF-02 | Phase 28 | Complete |
| DEF-03 | Phase 28 | Complete (UAT pending) |
| DEF-04 | Phase 28 | Complete (UAT pending) |
| DEF-05 | Phase 28 | Complete (UAT pending) |

**Coverage:**

- v0.5 requirements: 73 total
- Mapped to phases: 73
- Unmapped: 0 ✓

---
*Requirements defined: 2026-10-12*
*Last updated: 2026-10-15 after Phase 28 added (DEF-01..DEF-05)*
