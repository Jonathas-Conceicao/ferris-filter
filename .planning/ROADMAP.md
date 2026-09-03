# Roadmap: FerrisFilter

## Milestones

- ✅ **v0.1 Initial Release** — Phases 1-4 (shipped 2026-05-28) — [archive](milestones/v0.1-ROADMAP.md)
- ✅ **v0.2 Declarative Rule Model** — Phases 5-7 (shipped 2026-06-12) — [archive](milestones/v0.2-ROADMAP.md)
- ✅ **v0.3 Economy Items & Exceptional Gear** — Phases 8-10 (shipped 2026-06-27) — [archive](milestones/v0.3-ROADMAP.md)
- 🔄 **v0.4 Overlay Filter Editor Rework** — Phases 11-14 (started 2026-09-03)

## Phases

<details>
<summary>✅ v0.1 Initial Release (Phases 1-4) — SHIPPED 2026-05-28</summary>

- [x] Phase 1: Foundation (3/3 plans) — completed 2026-05-24
- [x] Phase 2: Static Generate (3/3 plans) — completed 2026-05-24
- [x] Phase 3: Pricing Layer (4/4 plans) — completed 2026-05-25
- [x] Phase 4: Sync (4/4 plans) — completed 2026-05-28

Full detail: [milestones/v0.1-ROADMAP.md](milestones/v0.1-ROADMAP.md)

</details>

<details>
<summary>✅ v0.2 Declarative Rule Model (Phases 5-7) — SHIPPED 2026-06-12</summary>

- [x] Phase 5: Rule Schema (2/2 plans) — completed 2026-06-05
- [x] Phase 6: Emitter Rewrite + Migration Parity (2/2 plans) — completed 2026-06-05
- [x] Phase 7: New Capabilities + Docs (3/3 plans) — completed 2026-06-12

Full detail: [milestones/v0.2-ROADMAP.md](milestones/v0.2-ROADMAP.md)

</details>

<details>
<summary>✅ v0.3 Economy Items & Exceptional Gear (Phases 8-10) — SHIPPED 2026-06-27</summary>

- [x] Phase 8: Sync Pipeline (2/2 plans) — completed 2026-06-24
- [x] Phase 9: Gem Background Styling (1/1 plan) — completed 2026-06-24
- [x] Phase 10: Corrupted Condition + Exceptional Gear (2/2 plans) — completed 2026-06-24

Full detail: [milestones/v0.3-ROADMAP.md](milestones/v0.3-ROADMAP.md)

</details>

### v0.4 Overlay Filter Editor Rework

- [ ] **Phase 11: Core Library Extraction** — split the compiler into a library so the overlay and the CLI are two front ends over one core
- [ ] **Phase 12: Dynamic Rule Store** — `dynamic.json` modeled, persisted, and compiled to a complete `dynamic.filter`, headless
- [ ] **Phase 13: Overlay Shell** — an always-on-top window over PoE2 with a global toggle hotkey
- [ ] **Phase 14: Rule Editor + Apply** — the editing UI, wired to the Phase 12 store

## Phase Details

### Phase 11: Core Library Extraction
**Goal**: The models, pricer and emitter live behind a library target that any binary can depend on, and the CLI becomes a thin shell over it — with `generate`, `sync` and `install` behaving exactly as they do today.
**Depends on**: Nothing (first phase of v0.4)
**Requirements**: CORE-01, CORE-02, CORE-03
**Success Criteria** (what must be TRUE):
  1. A second binary in this workspace can call the rule compiler and produce filter text without duplicating any model, pricer or emitter code.
  2. `artifact_guard` and `generate_snapshot` pass with **zero** changes to `tests/fixtures/seed-universal.filter` or the insta snapshot — the extraction is provably behavior-preserving.
  3. `generate`, `generate --early-game`, `generate --config`, `sync` and `install` produce the same output and exit codes as before the split.
  4. The CLI binary contains no tiering, policy or emit logic — only argument parsing, IO wiring and error reporting.
  5. `seed_prices` imports the real currency types instead of its local duplicates, and the duplicate definitions are gone.
**Plans**: 1-2 plans
**UI hint**: no

### Phase 12: Dynamic Rule Store
**Goal**: A persisted `dynamic.json` rule set compiles to a complete, playable `dynamic.filter` — policy blocks from the dynamic rules, economy sections from `prices.json` — entirely without a GUI, so the feature is verifiable on the project's existing testing terms.
**Depends on**: Phase 11
**Requirements**: DYN-01, DYN-02, DYN-03, DYN-04, DYN-05, DYN-06, DYN-07, DYN-08
**Success Criteria** (what must be TRUE):
  1. A `dynamic.json` containing an ordered rule array parses, round-trips, and reports a malformed field with its path — reusing the existing `Rule`/`When` types rather than new ones.
  2. Compiling a dynamic rule that sets only `sockets` emits a block containing `Sockets >= N` and **no** `Class`, `Rarity`, `BaseArmour`, `Quality` or `Corrupted` lines.
  3. Reordering two rules in `dynamic.json` reorders their blocks in `dynamic.filter`, confirming ordinal position drives precedence.
  4. A generated `dynamic.filter` contains the currency, gem and unique economy sections, matching what `universal.filter` emits from the same `prices.json`.
  5. No dynamic rule emits an `AreaLevel` line, while `config.json`'s area-level rules still do.
  6. Interrupting the write leaves the previous `dynamic.filter` intact — no truncated file is ever observable in the game folder.
  7. Edits written to `dynamic.json` are present after the process exits and restarts.
  8. `artifact_guard` and `generate_snapshot` still pass with no fixture changes.
**Plans**: 2 plans
**UI hint**: no

### Phase 13: Overlay Shell
**Goal**: A window renders above a running Path of Exile 2 and can be summoned and dismissed by a global hotkey without alt-tabbing — the delivery surface, with no editing in it yet.
**Depends on**: Phase 11
**Requirements**: SHELL-01, SHELL-02, SHELL-03
**Success Criteria** (what must be TRUE):
  1. With PoE2 running in windowed-fullscreen and focused, the window is visible on top of the game.
  2. A global hotkey shows and hides the window while the game holds focus.
  3. The window is opaque, does not pass mouse input through, and is smaller than the full screen — the configuration that avoids the documented egui/eframe Windows overlay defects.
  4. Closing the overlay leaves the game unaffected, and the overlay makes no attempt to read game memory or send input to it.
  5. The overlay binary is not named `overlay` (egui #5512 gives a package with that name a black background).
**Plans**: 1-2 plans
**UI hint**: yes

### Phase 14: Rule Editor + Apply
**Goal**: Every dynamic rule is editable from the overlay — style, classes, rarity, defences, sockets, quality, corrupted, each condition independently togglable — and Apply writes `dynamic.filter` and tells the player to reload it in-game.
**Depends on**: Phase 12, Phase 13
**Requirements**: EDIT-01, EDIT-02, EDIT-03, EDIT-04, EDIT-05, EDIT-06, EDIT-07, EDIT-08, EDIT-09
**Success Criteria** (what must be TRUE):
  1. A rule can be added, deleted and moved in the list, and moving it changes which of two overlapping rules wins in the emitted filter.
  2. Disabling a single rule covering the flask and charm classes removes flask drops from the generated filter — the milestone's motivating example, done in one click.
  3. The style dropdown offers exactly the styles defined in `config.json`, and picking one changes the emitted block's styling actions.
  4. The class picker's options come from `config.json`'s class lists — adding a class there makes it selectable without a code change.
  5. Ticking armour and energy shield while leaving evasion unticked emits `BaseArmour > 0`, `BaseEnergyShield > 0` and `BaseEvasion 0`; all seven PoE2 armour types are reachable this way, and disabling the defence group emits no defence lines at all.
  6. Turning any individual condition off removes exactly its line from the emitted block and leaves the others untouched.
  7. Apply writes `dynamic.filter` to the game folder and the UI states that the filter must be reloaded via Options → Game → Item Filter → Refresh.
  8. Rule mutation, validation and compilation are library functions covered by unit tests; the egui layer holds no logic that can only be exercised by clicking.
**Plans**: 2-3 plans
**UI hint**: yes

## Progress

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 1. Foundation | v0.1 | 3/3 | Complete | 2026-05-24 |
| 2. Static Generate | v0.1 | 3/3 | Complete | 2026-05-24 |
| 3. Pricing Layer | v0.1 | 4/4 | Complete | 2026-05-25 |
| 4. Sync | v0.1 | 4/4 | Complete | 2026-05-28 |
| 5. Rule Schema | v0.2 | 2/2 | Complete | 2026-06-05 |
| 6. Emitter Rewrite + Migration Parity | v0.2 | 2/2 | Complete | 2026-06-05 |
| 7. New Capabilities + Docs | v0.2 | 3/3 | Complete | 2026-06-12 |
| 8. Sync Pipeline | v0.3 | 2/2 | Complete | 2026-06-24 |
| 9. Gem Background Styling | v0.3 | 1/1 | Complete | 2026-06-24 |
| 10. Corrupted Condition + Exceptional Gear | v0.3 | 2/2 | Complete | 2026-06-24 |
| 11. Core Library Extraction | v0.4 | 0/? | Not started | - |
| 12. Dynamic Rule Store | v0.4 | 0/? | Not started | - |
| 13. Overlay Shell | v0.4 | 0/? | Not started | - |
| 14. Rule Editor + Apply | v0.4 | 0/? | Not started | - |

---
*v0.1 shipped 2026-05-28. v0.2 shipped 2026-06-12. v0.3 shipped 2026-06-27. v0.4 roadmap defined 2026-09-03.*

## Backlog

### Phase 999.1: Config-driven styles (no hardcoded vocabulary) (BACKLOG)

**Goal:** Any config should be able to define and use new named styles without a code change.
Today the style vocabulary is hardcoded in the `Styles` struct (`src/models/config.rs`) and
`emitter::resolve_style` (`src/emitter.rs`) — adding `craftable` (for `ssf_icewolf_config.json`)
required editing both. Replace the named-field `Styles` struct with a name→`Style` map (or keep
S–F/unchanged/hidden/unidentified as the known palette plus an open `extra` map), resolve styles
by lookup instead of a `match`, and validate at load-time that every `rule.style` resolves.
`FUTURE` comments already mark both code locations.
**Requirements:** TBD
**Plans:** 0 planned (not yet scoped)

Plans:
- [ ] TBD (promote with /gsd-review-backlog when ready)
