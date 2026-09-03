# Requirements: FerrisFilter — v0.4 Overlay Filter Editor Rework

**Defined:** 2026-09-03
**Core Value:** `generate` turns the config into a correct, deterministic PoE2 loot filter.

**Milestone goal:** FerrisFilter gains an in-game overlay that edits filter rules directly and
writes a `dynamic.filter` on demand, replacing pre-generated per-build presets.

**Acceptance bar (author-set):** `config.json → universal.filter` behaves exactly as it does today
— `artifact_guard` and `generate_snapshot` pass with **zero** fixture re-baselining across the
whole milestone. The dynamic path is additive.

## v0.4 Requirements

### Core Library (CORE)

- [ ] **CORE-01**: The crate exposes a library target that other binaries can depend on, covering
  the models, pricer and emitter; `generate`, `sync` and `install` keep their current behavior and
  the full existing test suite passes unchanged.
- [ ] **CORE-02**: The CLI binary is a thin shell over the library — no policy, tiering or emit
  logic lives in the binary.
- [ ] **CORE-03**: The `seed_prices` duplicated-currency-struct workaround (added in v0.1 because
  the crate had no library target) is removed in favor of importing the real types.

### Dynamic Rule Store (DYN)

- [ ] **DYN-01**: `dynamic.json` is modeled in serde as an ordered array of rules reusing the
  existing `Rule` / `When` types, round-trips, and reports validation failures with field paths.
- [ ] **DYN-02**: Rules are identified by ordinal position; there is no `name` field, and array
  order defines first-match-wins precedence.
- [ ] **DYN-03**: The dynamic rule set persists to disk in the repository directory and is reloaded
  on the next run, so edits survive restarts.
- [ ] **DYN-04**: Every `when` condition is independently optional — a condition absent from a
  dynamic rule emits no corresponding line in the generated block.
- [ ] **DYN-05**: Dynamic rules carry no `areaLevel` condition; `config.json` retains the field for
  the CLI path.
- [ ] **DYN-06**: `dynamic.json` compiles to a complete, playable `dynamic.filter` — the policy
  blocks from `dynamic.json`, plus the economy sections (currency, gems, uniques) from
  `prices.json` through the existing pricer, matching how `universal.filter` emits them.
- [ ] **DYN-07**: `dynamic.filter` is written atomically into the game folder (write-then-rename),
  so a partially written filter is never visible to the game.
- [ ] **DYN-08**: The whole dynamic compile is reachable headlessly, without the overlay, so it is
  unit- and integration-testable on the project's existing terms.

### Overlay Shell (SHELL)

- [ ] **SHELL-01**: An always-on-top window renders above Path of Exile 2 while the game runs in
  windowed or windowed-fullscreen mode.
- [ ] **SHELL-02**: A global hotkey toggles overlay visibility while the game holds focus, so it
  can be summoned without alt-tabbing.
- [ ] **SHELL-03**: The overlay window is opaque, non-passthrough, and never sized to the full
  screen — the configuration that avoids the documented egui/eframe Windows overlay defects.

### Rule Editor (EDIT)

- [ ] **EDIT-01**: The overlay lists the dynamic rules in ordinal order and supports adding,
  deleting and reordering them, with reordering changing match precedence.
- [ ] **EDIT-02**: A rule can be enabled or disabled, so one toggle suppresses a whole category —
  the "hide all flask drops" case.
- [ ] **EDIT-03**: Each rule has a style selector populated from the pinned `config.json` styles
  palette (S–F, `Recycle`, `exceptional`, `hidden`, `unchanged`, `unidentified`).
- [ ] **EDIT-04**: Each rule has a multi-select class picker whose options come from the class
  lists already in `config.json`, not a hardcoded list.
- [ ] **EDIT-05**: Each rule has a rarity selector emitting the existing compact comparator forms
  (`>=Magic`, `<=Rare`, `==Rare`, …).
- [ ] **EDIT-06**: Each rule has a three-checkbox defence picker (armour / evasion / energy shield)
  where a ticked defence emits `> 0` and an unticked one emits `0`, making all seven PoE2 armour
  types selectable; with the group disabled, no defence condition is emitted at all.
- [ ] **EDIT-07**: Each rule exposes sockets minimum, quality minimum, and a corrupted control
  whose unset state omits the condition entirely.
- [ ] **EDIT-08**: Every condition control can be toggled off individually, and a disabled control
  contributes nothing to the emitted block (the UI expression of DYN-04).
- [ ] **EDIT-09**: An Apply action writes `dynamic.filter` and tells the player to reload the
  filter in-game via Options → Game → Item Filter → Refresh.

## Future Requirements

Deferred — acknowledged, not in the v0.4 roadmap.

### Overlay / editor (next-milestone candidates)

- **EDITX-01**: Named preset save/load, for swapping between characters.
- **EDITX-02**: Undo/redo history for a session of edits.
- **EDITX-03**: `areaLevel` conditions exposed in the dynamic editor.
- **EDITX-04**: Editing economy tiers, cutoffs and the style palette from the overlay.
- **EDITX-05**: "What would this rule match" preview / simulation against a sample item set.
- **EDITX-06**: Detect and warn when PoE2 is running in exclusive fullscreen (where no overlay can
  draw).

### Carried from v0.3

- **ECONX-01**: BaseType-matched, economy-tiered Tablets category (class-less `BaseType ==`
  blocks; corrected base names). Needs pricer/emitter support for class-less category blocks.
- **GENX-01..04**: `--dry-run`, `--diff`, `-v/--verbose` rule logging, stale-`prices.json` warning.
- **SYNX-01..02**: Multiple build profiles; alternate price-source fallback.
- **STYLEX-01**: Config-driven style vocabulary (backlog Phase 999.1) — define new named styles in
  config without a code change.

## Out of Scope

Explicitly excluded for v0.4. Documented to prevent scope creep.

| Feature | Reason |
|---------|--------|
| Editing economy tiers / cutoffs in the overlay | Pinned this milestone (author directive). `dynamic.filter` *contains* the economy sections; it just does not let you edit them. |
| Editing the style palette in the overlay | Pinned this milestone. The style selector picks from the existing palette; it does not define styles. |
| `areaLevel` in the dynamic vocabulary | Live editing replaces level bands — that is the point of the rework. `config.json` keeps the field so the CLI path is unaffected. |
| Named presets / build profiles | The four `ssf_*_config.json` presets are what v0.4 removes. A single persisted live state replaces them; per-build differences become ordinary rule edits. |
| Undo history | Persistence across restarts was the actual need behind the question; undo was not requested. |
| Filesystem watching / auto-apply / "live sync" | PoE2 never re-reads a filter on its own and no external process can force it. Any such affordance would promise something the game cannot deliver. |
| Clipboard or OCR item reading | That is a price checker (Exiled Exchange 2's job), not a filter editor. |
| Transparent click-through overlay | Needed only for painting over the game world. An opaque panel avoids five of six documented egui Windows overlay defects. |
| ToS notices or disclaimers in the product | Filter generation tools are universal in this community; v0.4 changes *when* the filter is written, not what the tool touches. Author directive. |
| macOS / Linux support | Game folder resolution and overlay behavior here are Windows-shaped. |
| Re-baselining `seed-universal.filter` | The acceptance bar: if a fixture moves during this milestone, something is wrong. |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| CORE-01 | Phase 11 | Pending |
| CORE-02 | Phase 11 | Pending |
| CORE-03 | Phase 11 | Pending |
| DYN-01 | Phase 12 | Pending |
| DYN-02 | Phase 12 | Pending |
| DYN-03 | Phase 12 | Pending |
| DYN-04 | Phase 12 | Pending |
| DYN-05 | Phase 12 | Pending |
| DYN-06 | Phase 12 | Pending |
| DYN-07 | Phase 12 | Pending |
| DYN-08 | Phase 12 | Pending |
| SHELL-01 | Phase 13 | Pending |
| SHELL-02 | Phase 13 | Pending |
| SHELL-03 | Phase 13 | Pending |
| EDIT-01 | Phase 14 | Pending |
| EDIT-02 | Phase 14 | Pending |
| EDIT-03 | Phase 14 | Pending |
| EDIT-04 | Phase 14 | Pending |
| EDIT-05 | Phase 14 | Pending |
| EDIT-06 | Phase 14 | Pending |
| EDIT-07 | Phase 14 | Pending |
| EDIT-08 | Phase 14 | Pending |
| EDIT-09 | Phase 14 | Pending |

**Coverage:**
- v0.4 requirements: 23 total
- Mapped to phases: 23 (100%) ✓
- Unmapped: 0 ✓

---
*Requirements defined: 2026-09-03*
