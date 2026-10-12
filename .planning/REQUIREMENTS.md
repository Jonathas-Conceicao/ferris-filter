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

- [ ] **PARSE-01**: User can open any working PoE2 filter without error: FilterBlade/NeverSink,
  poe2filter.com, hand-written, or FerrisFilter output.
- [ ] **PARSE-02**: Every block type, condition, operator and action documented at
  pathofexile.com/item-filter/about is recognized, including `Continue`, `Import` and `Minimal`.
- [x] **PARSE-03
**: Saving a filter with no edits writes a byte-identical file. Line endings, BOM,
  indentation, comments and spacing are all preserved. This is verified against the real
  FilterBlade/NeverSink filter.
- [ ] **PARSE-04**: Saving after editing one block changes only that block's lines. Every other byte
  of the file is untouched.

### Rule Editor (EDIT)

- [ ] **EDIT-01**: User can view and edit every condition on any block: all operators, and
  multi-value lists. No condition is read-only.
- [ ] **EDIT-02**: User can view and edit every action on any block: colours, font size, alert
  sounds (built-in and custom), minimap icon, beam effect, and drop-sound toggles.
- [ ] **EDIT-03**: User can switch a block between `Show`, `Hide` and `Minimal`, and toggle
  `Continue`.
- [ ] **EDIT-04**: Disabling a rule comments its block out. Commented-out blocks in a loaded filter,
  such as NeverSink's, appear as disabled rules and can be enabled.
- [ ] **EDIT-05**: User can add, delete, duplicate and reorder rules.
- [ ] **EDIT-06**: User can apply a named style to a rule, which sets the rule's style actions.
- [ ] **EDIT-07**: User can search the rule list, and the editor stays responsive on a filter of
  about 460 blocks (the NeverSink sample).
- [ ] **EDIT-08**: Apply writes the edited filter, and Revert discards unsaved edits. This is the
  v0.4 model: edits are held in memory and Apply is the single writer.
- [ ] **EDIT-09**: Apply refuses to overwrite a filter that changed on disk after it was loaded,
  and tells the user why.
- [ ] **EDIT-10**: The first time FerrisFilter writes to a filter, it keeps a backup copy of the
  original.

### Economy Section (ECON)

- [ ] **ECON-01**: Economy-priced blocks are written after the general rules, inside a section
  delimited by comment markers.
- [ ] **ECON-02**: The economy section is not shown in the rule editor. The rule list shows a
  summary instead, e.g. "+ 42 locked rule blocks".
- [ ] **ECON-03**: Apply regenerates the economy section from `prices.json`, `tiers.json` and the
  styles, changing no byte outside the markers.
- [ ] **ECON-04**: User can add an economy section to a filter that has none (a new empty filter or
  a foreign filter), and can remove it.

### Filter Location (STORE)

- [ ] **STORE-01**: By default the overlay edits the game-folder `dynamic.filter` directly. There
  is no intermediate JSON store.
- [ ] **STORE-02**: User can select a different filter in the game folder from the configuration
  tab, and the choice is remembered across launches.
- [ ] **STORE-03**: A new filter starts empty, with an optional economy section.
- [ ] **STORE-04**: `dynamic.json`, `dynamic_base.json`, the overlay's JSON store, and CLI
  `apply --reset/--base/--store` are removed. CLI `apply` operates on the filter file. The breaking
  change is noted in `CHANGELOG.md`.
- [ ] **STORE-05**: `ff` can be launched from anywhere, including by double-clicking the exe. It
  finds the repo files (`styles.json`, `tiers.json`, `prices.json`, …) without relying on the
  process working directory.

### Styles (STYLE)

- [ ] **STYLE-01**: Named styles live in a committed repo-root `styles.json`. Adding a new style
  requires no code change.
- [ ] **STYLE-02**: A rule naming an unknown style, or an invalid `styles.json`, produces a clear
  error at load time.
- [ ] **STYLE-03**: A rule that uses a named style remembers it (via a comment tag in the filter).
  When the style changes, the rule's actions update on the next Apply.
- [ ] **STYLE-04**: `generate` output is unchanged: the `seed-universal.filter` fixture stays
  byte-identical.

### Configuration Tab (CONF)

- [ ] **CONF-01**: The overlay has a configuration tab, separate from the rule editor.
- [ ] **CONF-02**: User can add, edit and delete named styles: text, border and background colour
  (colour picker), font size, sound, minimap icon and beam. Built-in styles cannot be deleted.
- [ ] **CONF-03**: User can edit the economy tier cutoffs in `tiers.json` (not prices). Invalid
  cutoffs are shown inline and cannot be applied.
- [ ] **CONF-04**: The configuration tab has its own Apply and Revert, and Apply writes all of its
  files or none of them.

### Single Instance (INST)

- [ ] **INST-01**: Launching `ff` while it is already running brings up the running stripe (even
  when hidden to the tray) and exits, instead of opening a second instance.

## Future Requirements

Deferred. Tracked but not in the v0.5 roadmap.

- **Style rename** (with tag rewriting across the filter).
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
| EDIT-01 | Phase 20 | Pending |
| EDIT-02 | Phase 20 | Pending |
| EDIT-03 | Phase 19 | Pending |
| EDIT-04 | Phase 19 | Pending |
| EDIT-05 | Phase 19 | Pending |
| EDIT-06 | Phase 20 | Pending |
| EDIT-07 | Phase 19 | Pending |
| EDIT-08 | Phase 19 | Pending |
| EDIT-09 | Phase 18 | Pending |
| EDIT-10 | Phase 18 | Pending |
| ECON-01 | Phase 18 | Pending |
| ECON-02 | Phase 19 | Pending |
| ECON-03 | Phase 18 | Pending |
| ECON-04 | Phase 18 | Pending |
| STORE-01 | Phase 18 | Pending |
| STORE-02 | Phase 21 | Pending |
| STORE-03 | Phase 18 | Pending |
| STORE-04 | Phase 18 | Pending |
| STORE-05 | Phase 18 | Pending |
| STYLE-01 | Phase 17 | Pending |
| STYLE-02 | Phase 17 | Pending |
| STYLE-03 | Phase 20 | Pending |
| STYLE-04 | Phase 17 | Pending |
| CONF-01 | Phase 21 | Pending |
| CONF-02 | Phase 21 | Pending |
| CONF-03 | Phase 21 | Pending |
| CONF-04 | Phase 21 | Pending |
| INST-01 | Phase 22 | Pending |

**Coverage:**
- v0.5 requirements: 32 total
- Mapped to phases: 32
- Unmapped: 0 ✓

---
*Requirements defined: 2026-10-12*
*Last updated: 2026-10-12 after roadmap creation (traceability filled)*
