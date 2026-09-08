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

- [x] **Phase 11: Callable Compile API** — make the generate pipeline callable as a library API so the overlay can drive it without duplicating orchestration (completed 2026-09-03)
- [x] **Phase 12: Dynamic Rule Store** — `dynamic.json` modeled, persisted, and compiled to a complete `dynamic.filter`, headless (completed 2026-09-04)
- [x] **Phase 13: Overlay Shell** — an always-on-top window over PoE2 with a global toggle hotkey (completed 2026-09-06)
- [ ] **Phase 14: Rule Editor + Apply** — the editing UI, wired to the Phase 12 store
- [ ] **Phase 15: BaseType Lists and Rule Names** — a rule can match by base-type list alone, with `class` demoted to an optional condition, and can carry a name that shows in the editor and heads its filter block

## Phase Details

### Phase 11: Callable Compile API

**Goal**: The generate pipeline is callable as a library API — caller-supplied paths, a typed error enum, and the price-source decision returned as a value rather than printed — so the Phase 13/14 overlay can drive it without duplicating orchestration, depending on the working directory, or needing stdout.
**Depends on**: Nothing (first phase of v0.4)
**Requirements**: CORE-01, CORE-02, CORE-03, CORE-04
**Note**: Scope corrected during `/gsd-discuss-phase 11` (2026-09-03). The roadmap was written believing the crate had no library target; it has had one for some time — `[lib]` is declared, `src/lib.rs` exports every module, and both binaries are already ~6-line shims over `ferrisfilter::run()`. The stale claim came from a v0.1 STATE.md note and `seed_prices.rs`'s header comment. The real blocker is that `run::cmd_generate` is private, hardcodes CWD-relative paths, writes its own output and prints warnings — so no second front end can reuse it. See `11-CONTEXT.md`.
**Success Criteria** (what must be TRUE):

  1. A caller outside the CLI can load inputs and produce filter text through public library functions, passing its own paths, without reimplementing the price-source fallback, rate derivation or tier pricing.
  2. No library code reads a hardcoded working-directory-relative input path; every input path originates from the caller.
  3. Expected failure paths — unreadable/malformed config, malformed prices, unreadable/malformed/invalid tiers, reference-rate failure, unknown style — are represented as distinct, matchable variants of one error type returned by the library.
  4. `tests/cli_errors.rs` passes **unmodified**: the rendered stderr still contains `Cannot read`, `config.json`, `styles.S.font`, `expected u8`, `missing required tier` and `not strictly ascending`, with miette source spans intact.
  5. Which price source was used (live / committed fallback pair / none) is available to the caller as a value; the CLI derives its existing warning text from it and its stderr output is unchanged.
  6. `artifact_guard` and `generate_snapshot` pass with **zero** changes to `tests/fixtures/seed-universal.filter` or the insta snapshot, and the full suite passes — the refactor is provably behavior-preserving.
  7. `generate`, `generate --early-game`, `generate --config`, `sync` and `install` produce the same output, stderr and exit codes as before.
  8. `seed_prices` imports the real types from the library, its five duplicate structs are gone, and its comment claiming the crate has no `[lib]` target is removed.

**Plans**: 2 plans

  - [x] 11-01-PLAN.md — typed loader errors; new src/compile.rs two-stage API (InputPaths, PriceSourcePolicy, PriceSource, CompileError, load_inputs, compile); CLI rewired and proven byte-identical across 6 cases
  - [x] 11-02-PLAN.md — seed_prices imports the library models; five mirror structs and the stale "no [lib] target" comment removed; output byte-identical

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

  - [x] 12-01-PLAN.md — Rule.enabled + disabled-rule suppression preserving ordinal labels; FilterIdentity/generate_as/compile_as; dynamic_base.json cleaned for ordinal identity
  - [x] 12-02-PLAN.md — src/dynamic.rs lifecycle (seed/reset/compile/atomic write); `apply` subcommand; tests/dynamic_filter.rs

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

**Plans**: 5 plans

  - [x] 13-01-PLAN.md — Cargo/binary restructure (ff = GUI, ff-cli = console) + the always-on-top, opaque, sub-screen stripe window
  - [x] 13-02-PLAN.md — Ferris tray icon with Show/Quit menu + global Shift+Z registration, with registration failure surfaced in the tray
  - [x] 13-03-PLAN.md — Pure visibility state machine + raw-HWND SW_SHOWNOACTIVATE/SW_HIDE reveal and hide (summon never steals focus, click-away hides)
  - [x] 13-04-PLAN.md — Read-only dynamic.json rule list (pure, tested view-model) + PoE2 Options palette, Cinzel font and header plate
  - [x] 13-05-PLAN.md — In-game manual verification checklist for all five success criteria + windowed-mode documentation

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

**Plans**: 4 plans

Plans:

- [x] 14-01-PLAN.md — pure mutation layer (RuleId/WorkingRules + every condition set/clear), atomic store::save, and the flask/reorder end-to-end tests
- [x] 14-02-PLAN.md — the accordion, painted toggle/arrow controls, add/delete/reorder/enable, and the per-frame autosave
- [x] 14-03-PLAN.md — the condition editors: style, classes, rarity, defences, sockets, quality, corrupted
- [x] 14-04-PLAN.md — Apply + reload instruction + unapplied indicator, CLAUDE.md, and the manual in-game checklist

**UI hint**: yes

### Phase 15: BaseType Lists and Rule Names

**Goal**: A rule can match on a raw `BaseType` list with no `Class` line at all — `class` becomes an
optional condition like every other, so turning it off both enables base-type-only rules and makes a
freshly added rule open clean. A rule can also carry an optional name, shown in the editor's row
header and emitted as its block comment.
**Depends on**: Phase 14
**Requirements**: EDIT-10, EDIT-11
**Success Criteria** (what must be TRUE):

  1. A rule with a base-type list and no classes emits a block with a `BaseType ==` line and **no**
     `Class` line — the author's hand-written *Change Bases* rule is expressible in the editor.
  2. `class` has the same independent off switch every other condition has (EDIT-08's inventory),
     and its picker is hidden when off.
  3. A named rule emits `Show # <name>`; an unnamed one still falls back to the derived `rule {idx}`
     label, so existing stores are unaffected.
  4. A rule's name shows in the overlay row header, without becoming a second identity scheme —
     `RuleId` stays the identity and the array index stays the precedence (PITFALLS.md §9).
  5. `config.json → universal.filter` is byte-for-byte unchanged; no fixture re-baselining.

**Plans**: 3 plans

Plans:

**Wave 1**
- [ ] 15-01-PLAN.md — the pure layer: base-type parse/add/remove/clear, set_name/clear_name/clear_class, the name in the row header and the Bases label in the summary

**Wave 2** *(blocked on Wave 1 — calls the `edit::` functions Wave 1 creates)*
- [ ] 15-02-PLAN.md — the editor controls: the id-keyed ephemeral panel-open state, the name field, the class off switch, and the item-by-item base-type list

**Wave 3** *(blocked on Wave 2 — documents and verifies what Wave 2 ships; ends in a blocking human checkpoint)*
- [ ] 15-03-PLAN.md — CLAUDE.md corrected and extended, the four-item in-game checklist, and the blocking live check

**Cross-cutting constraints** (`must_haves.truths` shared across plans):
- `config.json → universal.filter` stays byte-for-byte unchanged; `src/emitter.rs` and
  `src/models/config.rs` are not modified. Asserted by every task in all three plans.
- The author's *Change Bases* rule — a base-type list with no `Class` line — must be representable
  as data (15-01), buildable in the editor (15-02), and land in `dynamic.filter` from a running
  game (15-03). The same requirement, proven at three different levels.
- Rendering a rule body writes nothing to the rule by itself: no auto-filled name, no auto-selected
  class, no auto-added base type (D-09/D-11). Enforced in 15-02 and re-checked by hand in 15-03.

**UI hint**: yes

**Groundwork already in place** (verified 2026-09-08, before planning): `When::base_type` and
`Rule::name` both already exist in the model and are already emitted — `src/models/config.rs:341`
and `:373`, `src/emitter.rs:418` and `:336-345`. Both features are expected to be overlay-side
work, not model or emitter work.

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
| 11. Callable Compile API | v0.4 | 2/2 | Complete | 2026-09-03 |
| 12. Dynamic Rule Store | v0.4 | 2/2 | Complete | 2026-09-04 |
| 13. Overlay Shell | v0.4 | 5/5 | Complete   | 2026-09-06 |
| 14. Rule Editor + Apply | v0.4 | 4/4 | Verification pending |  |
| 15. BaseType Lists and Rule Names | v0.4 | 0/3 | Planned |  |

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
**Plans:** 3/4 plans executed

Plans:

- [ ] TBD (promote with /gsd-review-backlog when ready)

### Phase 999.2: Overlay single-instance guard (BACKLOG)

**Goal:** Launching `ff` while it is already running should surface the existing stripe instead of
opening a second one. Discovered by the author during Phase 13 wave-2 verification (2026-09-04):
a second launch opens a second stripe window, and only the first instance owns the `Shift+Z`
registration — the second correctly reports the conflict in its tray tooltip (D-10), so the
failure is visible rather than silent, but two stripes and one working hotkey is not the intended
shape for a tray app that gets double-clicked. Fix shape: acquire a named mutex at startup; if it
is already held, signal the running instance to show itself and exit rather than creating a
window. Small, self-contained, and touches only the overlay entry point.
**Requirements:** TBD
**Plans:** TBD

Plans:

- [ ] TBD (promote with /gsd-review-backlog when ready)
