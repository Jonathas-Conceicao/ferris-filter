---
gsd_state_version: 1.0
milestone: v0.6
milestone_name: New PoE2-based Tool UI
status: executing
stopped_at: Completed 31-06-PLAN.md
last_updated: "2026-10-16T13:09:59.794Z"
last_activity: 2026-10-16
progress:
  total_phases: 7
  completed_phases: 2
  total_plans: 13
  completed_plans: 12
  percent: 9
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-10-16)

**Core value:** `generate` turns the config into a correct, deterministic PoE2 loot filter.
**Current focus:** Phase 31 — rules-tab-on-the-skin

## Current Position

Phase: 31 (rules-tab-on-the-skin) — EXECUTING
Plan: 7 of 7
Status: Ready to execute
Last activity: 2026-10-16

Progress (v0.6): [█░░░░░░░░░] 14%

## Performance Metrics

**Velocity:**

- Total plans completed: 118 (v0.1: 14 · v0.2: 7 · v0.3: 5 · v0.4: 16 · v0.5: 71)

**By Milestone:**

| Milestone | Phases | Plans | Commits | Span |
|-----------|--------|-------|---------|------|
| v0.1 Initial Release | 4 | 14 | 127 | 7 days |
| v0.2 Declarative Rule Model | 3 | 7 | ~38 | 7 days |
| v0.3 Economy Items & Exceptional Gear | 3 | 5 | 54 | 15 days |
| v0.4 Overlay Filter Editor Rework | 5 | 16 | 146 | 14 days |
| v0.5 Generic Filter Editor & Initial App Configuration | 13 | 71 | 452 | 4 days |

Per-plan timings for archived milestones live in the phase SUMMARY.md files under
`.planning/phases/` (see also `.planning/RETROSPECTIVE.md` for cross-milestone trends).
| Phase 13 P01 | 45min | 3 tasks | 8 files |
| Phase 13 P02 | 10min | 3 tasks | 6 files |
| Phase 13 P03 | 15min | 3 tasks | 4 files |
| Phase 13 P04 | 35min | 3 tasks | 9 files |
| Phase 13 P05 | ~16min | 2 tasks | 7 files |
| Phase 14 P01 | 55min | 3 tasks | 4 files |
| Phase 14 P02 | 70min | 3 tasks | 2 files |
| Phase 14 P03 | 65min | 3 tasks | 3 files |
| Phase 14 P04 | ~90min + 2-day fix stream | 3 tasks | 15 files |
| Phase 16 P02 | 20m | 2 tasks | 3 files |
| Phase 16 P03 | 30min | 2 tasks | 9 files |
| Phase 17 P02 | 25min | 2 tasks | 5 files |
| Phase 17 P04 | 20min | 3 tasks | 7 files |
| Phase 18 P03 | 25min | 2 tasks | 4 files |
| Phase 18 P04 | 15min | 2 tasks | 2 files |
| Phase 18 P05 | 25min | 2 tasks | 4 files |
| Phase 19 P02 | 25min | 2 tasks | 4 files |
| Phase 20 P01 | 25min | 3 tasks | 5 files |
| Phase 21 P01 | 15min | 2 tasks | 3 files |
| Phase 21 P02 | 20min | 2 tasks | 4 files |
| Phase 21 P03 | 15min | 2 tasks | 6 files |
| Phase 21 P04 | 25min | 2 tasks | 2 files |
| Phase 21 P05 | 35min | 2 tasks | 4 files |
| Phase 21 P06 | 45min | 3 tasks | 7 files |
| Phase 23 P02 | 15min | 2 tasks | 2 files |
| Phase 23 P03 | 25min | 2 tasks | 3 files |
| Phase 23 P04 | 10min | 2 tasks | 3 files |
| Phase 28 P02 | 25min | 2 tasks | 5 files |
| Phase 28 P03 | 10min | 2 tasks | 5 files |
| Phase 28 P04 | 6 min | 2 tasks | 4 files |
| Phase 30 P01 | 40min | 3 tasks | 8 files |
| Phase 30 P02 | ~2h | 3 tasks | 12 files |
| Phase 30 P03 | 60min | 2 tasks | 6 files |

## Accumulated Context

### Roadmap Evolution

- v0.6 roadmap created (2026-10-16): Phases 29-35 — workspace + ff-art from the root, skinned
  shell, RULES on the skin + add-at-top, dividers + economy banner, style table/editor + colour
  picker, ECONOMY + ABOUT tabs (tiers move; add/remove economy section lives on ECONOMY, after the
  Phase 32 banner work), prototype retirement + docs. UI phases 30-34 carry `UI hint: yes`.

- Phase 28 added (2026-10-15): **Default config files** — committed `styles-default.json` /
  `tiers-default.json`; live `styles.json` / `tiers.json` git-ignored and falling back to the
  defaults when absent (DEF-01..DEF-05). Run autonomously.

- Phase 27 added (2026-10-15): **UAT Feedback Fixes 4** — the author's fourth v0.5 UAT round
  (UAT4-01..UAT4-04): right-aligned Custom styles Order column, colour dropdown scroll bar fix,
  Simple Defences condition beside Defences, UAT results of Phases 16 and 23-26 recorded. Run
  autonomously.

- Phase 26 added (2026-10-14): **UAT Feedback Fixes 3** — the author's third v0.5 UAT round
  (UAT3-01..UAT3-08): grouped styles table, stable style ids + custom-style rename, in-memory style
  picker, no rule collapse, Defences card, Between + no duplicate conditions, collapsed picker,
  Quit confirm with unapplied edits. Run autonomously.

- Phase 25 added (2026-10-14): **UAT Feedback Fixes 2** — the author's second v0.5 UAT round
  (UAT2-01..UAT2-10): rule-body layout, Sockets slider, CONFIG styles table, one all-or-nothing
  Apply covering config and rules. Run autonomously.

- Phase 24 added (2026-10-13): **UAT Feedback Fixes** — thirteen overlay fixes from the author's
  hands-on v0.5 UAT (UAT-01..UAT-13), run autonomously.

- Phase 23 added (2026-10-13): **Auto-Reload Filter in Game on Apply** — after a RULES Apply that
  writes the filter, send `/reloaditemfilter` to a running game (RELOAD-01). Added after the v0.5
  milestone audit at the author's request.

- Phase 15 added (2026-09-08): **BaseType Lists and Rule Names** — raw `BaseType` list input with
  `class` demoted to an optional condition, and optional per-rule names shown in the editor and
  emitted as the block comment. Requested by the author explicitly **before v0.4 closes**, so the
  milestone audit waits on it. Detail and groundwork under *Requested, Not Yet Scoped* below.

### Decisions

Full decision log lives in PROJECT.md → Key Decisions. Decisions that still constrain future
work (carried forward past the v0.3 close):

- **PoE2 boolean conditions take no operator** — `Corrupted False`, never `Corrupted == False`.
- **OR across conditions must be separate rules** — filter blocks are AND-only.
- **Per-category style overrides ride on the tier style** via clone-null-append (clone the tier
  `Style`, null the overridden attribute, `emit_style_actions`, then append the override line).
  Used by priced uniques (bronze bg) and priced gems (per-category bg).

- **New named styles are added as `Option<Style>`** (like `craftable`, `exceptional`) so
  `--config` files predating the style keep parsing.

- **New optional `when` fields use `serde(default)` + `skip_serializing_if`** for zero-cost
  backward compatibility and clean round-trips.

- **Each uncut gem level is a distinct PoE2 base type** — never strip the level from `BaseType`.
- **Fixtures are regenerated hermetically** (temp dir + binary by absolute path, mirroring
  `generate_on_seed()`); never `cargo run` from the repo root, which picks up a live
  `prices.json`.

- **The style vocabulary is still hardcoded** in `Styles` + `emitter::resolve_style`; `FUTURE`
  comments mark both sites. Backlog Phase 999.1.

- [Phase 13]: eframe 0.36.1's App trait uses fn ui(&mut self, ui, frame), not update(ctx, frame) as sketched in the plan — corrected against installed crate source
- [Phase 13]: Overlay stripe docks right with EDGE_MARGIN=12.0px; dock_rect hard-clamps width/height strictly under the monitor size regardless of requested width or monitor size
- [Phase 13]: Local nightly rustc toolchain updated (1.90 to 1.100) to meet eframe/egui 0.36.1's rustc 1.95 MSRV; no repo-level rust-toolchain.toml pin exists yet
- [Phase 13]: Task 3's checkpoint cleared via live author verification on real hardware; all seven steps passed including the Shift+Z double-fire check
- [Phase 13]: No single-instance guard exists or is planned for Phase 13 -- a second ff opens a second stripe window; deferred, out of scope for SHELL-02
- [Phase 13]: Task 3's checkpoint (13-03) cleared via live author verification on real hardware -- repeated Shift+Z summons never steal keyboard focus (closes RESEARCH Open Question 2 negatively)
- [Phase 13]: Header close-control glyph (U+2715) renders as tofu box in egui's default font -- deferred to 13-04, which will paint two strokes with egui::Painter instead of substituting a Unicode glyph (the planned Cinzel face also lacks U+2715)
- [Phase 13]: Cinzel Regular fetched from Google's CSS delivery API (legacy-browser User-Agent) rather than google/fonts' GitHub repo, which now only ships a variable font that ab_glyph cannot reliably instantiate to one weight
- [Phase 13]: dynamic.json loads read-only exactly once at OverlayApp::new() startup -- no reload mechanism yet, since nothing in src/overlay/ can mutate the store this phase (D-20)
- [Phase 13]: Hotkey rebound from Shift+Z to Ctrl+Shift+Z at the author's request during live in-game verification, to keep Shift+Z free for other bindings (commit 15bff2f)
- [Phase 13]: Game-dimming behind the overlay stripe was requested during in-game verification and then DECLINED by the author (2026-09-06, “not really necessary”) -- closed, not deferred. It would have needed a second full-screen translucent window, conflicting with SHELL-03/D-12/D-13 opaque/non-passthrough/sub-screen constraints. The reasoning is retained only so a future request for the same thing starts from the constraint.
- [Phase 14]: set_class collapses a fully-deselected class list to None (never Some(vec![])) to avoid a malformed bare Class line the emitter has no emptiness guard against
- [Phase 14]: RuleId is a session-only, never-serialized identity layer over Vec<Rule> (overlay::edit::WorkingRules) -- the "currently expanded" rule state survives array reordering by construction, closing PITFALLS.md §9
- [Phase 14]: toggle_enabled collapses both None and Some(true) to None on re-enable -- re-enabling a rule never writes a redundant "enabled": true key
- [Phase 14]: overlay::store::save writes atomically via tempfile::NamedTempFile::new_in + persist (same discipline as dynamic::write_filter); store.rs's module header now documents the Phase 14 write boundary instead of Phase 13's now-stale "overlay never writes" claim
- [Phase ?]: 14-02: Deferred-action rendering -- row controls only record a RowAction during the render pass; the mutation happens once after the loop.
- [Phase ?]: 14-02: Quit is only requested during tray-menu polling; ViewportCommand::Close is deferred until after the body renders and autosave runs, so the last edit is never lost on Quit (RESEARCH Risk 4).
- [Phase ?]: Threaded RuleId + &Config + &mut Rule into the accordion body via WorkingRules::rule_mut, distinguishing immediate per-rule condition edits from the deferred RowAction path used for row-structural changes
- [Phase ?]: Comparator/value pickers (rarity op, rarity value, corrupted value) use plain-English labels (Exactly/At Least/At Most, Normal/Magic/Rare/Unique, False/True) instead of symbolic >=/<=/== labels, keeping the anti-comparator-string grep meaningful against the file's actual widget labels
- [Phase 14]: **D-02 REVERSED** (2026-09-08, author request after live use) -- edits are held in memory and Apply is the single writer, with a Revert to discard. The autosave model D-02 originally chose is preserved in 14-CONTEXT.md as superseded. Accepted trade-off: unsaved edits die with the process; what replaces the crash-safety is visibility (the indicator) plus deliberate discard (Revert).
- [Phase 14]: Apply persists the store and THEN compiles from it -- load-bearing order, because `dynamic::compile_store` reads the store from disk. Compiling first reintroduces the stale-Apply off-by-one fixed in 3375ad4 through a different route.
- [Phase 14]: The `unidentified` style is renamed `kite`, after its minimap icon -- the old name described the rules that used it rather than what it does. Breaking for any config naming the old name (unknown style is a hard generate-time error, by design).
- [Phase 14]: A fully-selected class group summarises as "Any <group>", not "All" -- a summary reads as the rule's own matching test, and any one class in the list satisfies it (author, reversing the first cut the same day).
- [Phase 14]: Running the overlay holds a lock on `target\debug\ff.exe`, so `cargo test` fails with "Access is denied" while the stripe is up (cargo builds all bin targets for integration tests). Use `cargo test --lib`, or point CARGO_TARGET_DIR at a scratch path.
- Phase 16-01: parser fixtures use scoped -text (tests/fixtures/parser/*.filter), not global
- Phase 16-02: keyword table is 80 entries, Ops None|Compare, lenient shape validation (no range/name checks)
- 16-03: block metadata accessors live on Document (block_meta/meta_get); disabled-block reasons are Info diagnostics, never Error
- 16-04: edits rebuild whole index and remap BlockIds by position; edit refused if block count/header/owned length/enabled state would change
- [Phase 19]: 19-02: structural ops go through commit_structural (explicit ids per block); moves never cross economy markers
- [Phase 19-03]: Only Error diagnostics in enabled non-economy blocks block Apply; rules-only Apply allowed via EconomyOp::Keep
- [Phase 20]: 20-01: Whole patch refuses only new Errors; other line ops refuse new Errors and Warnings; set_continue/set_block_keyword stay lenient on invalid disabled blocks
- [Phase 20-07]: Condition cards: value controls on line 2; list shapes and Rarity start as pending rows; opaque lines are raw Whole-edit rows; refused raw edits surface via ListUi.line_error
- [Phase 21]: Spaced style names allowed (tag round trip proven); render_tiers never sorts
- [Phase 21]: remove_meta works on disabled blocks; dangling style tags dropped there too
- [Phase 21]: Config colour picker is opaque; show_sections owns its ScrollArea; Keep editing dismisses the outside-edit guard
- [Phase 21]: CONFIG Apply marks filter out of date only when a file was written; RELOAD keeps config edits unless root changes
- [Phase 23]: Reload toggle persists on change (settings::save) and never marks CONFIG dirty; interim reload line replaced only if still displayed
- [Phase 28]: 28-02: reads go through InputPaths::styles_source/tiers_source (live, else -default.json); app-root marker is styles-default.json
- [Phase 28]: 28-03: overlay config files keep two baselines - content (source read) and live (live file bytes or None, the changed-on-disk guard)
- [Phase 28]: 28-04: styles.json/tiers.json untracked (index only) and git-ignored; committed -default.json files hold the defaults
- [Phase 30]: Config Revert uses an interim ff_modal Discard confirm until the popup plan

### Pending Todos

- **Amend `10-01-SUMMARY.md` prose** — it says `exceptional` is a required `Styles` field; it
  landed as `Option<Style>`. Code is correct and tested; only the prose is stale.

- ~~**Test-only compiler warnings**~~ — RESOLVED 2026-09-17 (`10ffe65`). Both went with the
  `seed_prices` cleanup: the emitter's unreachable match arm and `sync/metadata.rs`'s unused
  `FakeClient::ok`. Carried as tech debt since the v0.3 audit; the build is now warning-free.

- **Untracked working-tree files** — decide whether the `ssf_*_config.json` / `ssf_*.filter`
  per-build filters are tracked deliverables or `.gitignore` fodder, and drop the `scratch_*`,
  `foo.filter`, `temptest_gen.filter` scratch artifacts.

- **`Cargo.toml` is still `version = "0.1.0"`** while tags read v0.1/v0.2/v0.3 — confirm whether
  the crate version is meant to track milestone tags.

### Blockers/Concerns

None open.

### Quick Tasks Completed

| # | Description | Date | Commit | Directory |
|---|-------------|------|--------|-----------|
| 261015-nbp | Simple Defences: multi-select base types, written as grouped blocks | 2026-10-15 | 52ff64e | [261015-nbp-simple-defences-multi-select-base-types-](./quick/261015-nbp-simple-defences-multi-select-base-types-/) |
| 261015-tz2 | PoE2-style UI prototype: ff-art asset tool, skin module, ff-proto binary | 2026-10-16 | 206c6bb | [261015-tz2-poe2-style-ui-prototype-ff-art-asset-too](./quick/261015-tz2-poe2-style-ui-prototype-ff-art-asset-too/) |
| 261016-1de | PoE2 prototype round 2: game-like tabs, style editor, ABOUT tab, one-line status, trimmed minimap atlas | 2026-10-16 | f64cd42 | [261016-1de-poe2-prototype-tab-layout-style-editor-a](./quick/261016-1de-poe2-prototype-tab-layout-style-editor-a/) |

## Phase 15 Source Material — the author's two requests

Asked for by the author on **2026-09-08**, explicitly **before v0.4 closes**, and now scoped as
**Phase 15** in ROADMAP.md. No code work has started. Both are editor-side features; the groundwork
below was confirmed by reading the code, not assumed. This is the raw request as given — feed it to
`/gsd-discuss-phase 15` rather than re-deriving it.

### R1 — Raw-text `BaseType` list, with `class` demoted to an optional condition

Target output, which the author has been hand-writing into filters:

```
Show # Change Bases
  Rarity == Normal
  BaseType == "Heavy Belt" "Utility Belt" "Armoured Cap" "Sacred Focus"
```

Note there is **no `Class` line** — that is the point of the request. Today the overlay's rule
body always renders the class picker, so a base-type-only rule cannot be expressed there.

**What already exists (verified 2026-09-08):**

- `When::base_type: Option<Vec<String>>` is already in the model (`src/models/config.rs:341`),
  already `skip_serializing_if`, and already emitted as
  `BaseType == "A" "B"` by the emitter (`src/emitter.rs:418`).

- `overlay::view_model`'s `summarize_list` already documents itself as rendering "a name list
  (class or base type)".

**So the work is overlay-side:** a text-entry control for the list, and giving `class` the same
enable/disable treatment every other condition already has (`EDIT-08`'s off-switch inventory in
`rule_editor.rs` covers seven conditions; class is the one that is always-on). The author's second
motive is that this also makes a freshly added rule cleaner — a new rule currently opens with the
full class picker expanded.

**Open questions for the discussion:** how the raw list is parsed (one per line, or
comma/quote-separated?); whether typos are validated against anything (poe2db base names are not
in the repo) or accepted verbatim; and whether an empty list collapses to `None` the way
`edit::set_class` already does, to avoid emitting a malformed bare `BaseType` line.

### R2 — Optional per-rule `name`, shown in the editor and written to the filter

Wanted as `Show # rule 0 - Exceptional bases`, primarily so rules are easier to identify in the
overlay's own list.

**What already exists (verified 2026-09-08):**

- `Rule::name: Option<String>` is already in the model (`src/models/config.rs:373`).
- The emitter already writes `{Show|Hide} # {label}` where `label` is `rule.name` falling back to
  a derived `"rule {idx}"` (`src/emitter.rs:336-345`). The author's desired
  `# rule 0 - Exceptional bases` is just a `name` of `"rule 0 - Exceptional bases"`.

**So the work is again overlay-side:** a name field in the rule body, and using the name in the
row header where `row_header_text` currently shows `{ordinal}  {style}`.

**Watch out:** `dynamic_base.json` deliberately ships **without** rule names — `CLAUDE.md` states
dynamic rules are identified by their **index in the `rules` array**, and `PITFALLS.md` §9 is about
ordinal identity being fragile. Adding names must not quietly become a second identity scheme; the
session-only `RuleId` stays the identity, and a name is a label. Worth deciding explicitly whether
a name auto-populates with the derived `"rule N"` (which would then go stale on reorder) or stays
empty until typed.

**Route:** Phase 15 exists in ROADMAP.md as of 2026-09-08. Next is `/gsd-discuss-phase 15`, then
plan and execute. Phase 14 can be verified and closed independently first.

## Registered, Not Fixed

Found while working, deliberately left alone rather than guessed at (2026-09-10).

- **The overlay never says which price source it used.** `PriceSourcePolicy::Auto` silently falls
  back to the committed `prices-fallback.json` pair when `prices.json` is absent or empty. The CLI
  warns on stderr; the overlay has no console (D-17), so a player can Apply a fallback-priced
  filter believing it is live-priced. The plumbing already exists and is unused:
  `CompiledFilter.price_source` is documented as "returned, not printed, so the caller decides how
  to surface it" (D-09), and `grep price_source src/overlay/` finds nothing. A status line beside
  the Apply confirmation would close it. Author, 2026-09-09: not a priority, exceptional scenarios
  can wait.

- **`Breachstone` is emitted as `Stackable Currency` but repoe-fork says `QuestItem`.** Found by
  the same audit that caught Vaal Siphoner (2 of 1347 priced items disagree; the other was fixed).
  Not corrected because the right filter `Class` for it is not confirmable from repoe alone, and a
  wrong guess would break a block that currently works. Needs one in-game check, then a
  `classOverrides` entry in `mapping.json` — the mechanism is already there.

- **The overlay league is pinned to `Forbidden Rites` in `sync::OVERLAY_LEAGUE` and must be
  updated each league.** poe.ninja's `indexed` flag flips between leagues — observed reversing
  within six hours on 2026-09-10/11 — so automatic resolution cannot identify the current league.
  A picker populated from index-state would remove the maintenance burden; deferred deliberately
  rather than bolted onto a bug fix. `sync_economy_blocking` uses automatic league
  resolution, with no equivalent of the CLI's `--league`. That flag exists because poe.ninja can
  flag a live league as not indexed, which is exactly what happened with Forbidden Rites. If the
  auto-pick cannot see the current league, the overlay button will fail with the resolver's message
  and the CLI is the workaround.

## Deferred Items

Items acknowledged and deferred at the **v0.5** close on 2026-10-15 (open-artifact audit):

| Category | Item | Status |
|----------|------|--------|
| uat | Phase 14: 14-UAT.md | diagnosed, 0 pending scenarios (v0.4 artifact) |
| uat | Phase 16: 16-HUMAN-UAT.md | partial pass accepted, 0 pending; rest in backlog 999.12 |
| tech_debt | v0.5 audit W1-W4 and accepted review findings | See milestones/v0.5-MILESTONE-AUDIT.md |

Items acknowledged and deferred at the **v0.3** close on 2026-06-27 (open-artifact audit was
clear — these are scope/debt carries, not open GSD artifacts):

| Category | Item | Status |
|----------|------|--------|
| docs | `10-01-SUMMARY.md` prose vs `Option<Style>` reality | Cosmetic; code self-consistent and tested |
| scope | ECONX-01 Tablets as a class-less priced `BaseType ==` category | Deferred; needs pricer/emitter support + a poe.ninja spike |
| scope | Config-driven style vocabulary | Backlog Phase 999.1 |
| scope | Generate QoL (GENX-01..04): `--dry-run`, `--diff`, verbose rule logging, stale-prices warning | Deferred to a later milestone |
| scope | Sync extensions (SYNX-01..02): multiple build profiles, alternate price source | Deferred to a later milestone |

Items carried from the **v0.1** close on 2026-05-28:

| Category | Item | Status |
|----------|------|--------|
| uat | Live softcore no-flag `sync` happy path | Blocked on the PoE2 league cycle; tool fails safe. Live integration confirmed via `sync --standard`. Re-confirm on a new league launch. |
| verification | Live network sign-off for `sync` | Signed off 2026-05-28; full live re-check pending an active softcore league. |
| sync | Lenient-parse guard for Standard unique endpoints (`UniqueSanctumRelics` missing `core`) | Non-blocking; wrote 0 uniques on Standard. |
| sync | Optional no-flag `sync` → `--standard` fallback/hint when no challenge league is active | Enhancement idea. |

## Session Continuity

Last session: 2026-10-16T13:09:59.785Z
Stopped at: Completed 31-06-PLAN.md
Resume file: None

**Next step:** `/gsd-discuss-phase 29` (or `/gsd-plan-phase 29`).
