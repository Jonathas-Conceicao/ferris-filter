---
gsd_state_version: 1.0
milestone: v0.5
milestone_name: Generic Filter Editor & Initial App Configuration
status: executing
stopped_at: Completed 17-01-PLAN.md
last_updated: "2026-10-12T23:53:21.791Z"
last_activity: 2026-10-12
progress:
  total_phases: 17
  completed_phases: 1
  total_plans: 8
  completed_plans: 5
  percent: 6
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-10-12)

**Core value:** `generate` turns the config into a correct, deterministic PoE2 loot filter.
**Current focus:** Phase 17 — config-driven-styles

## Current Position

Phase: 17 (config-driven-styles) — EXECUTING
Plan: 2 of 4
Status: Ready to execute
Last activity: 2026-10-12

Progress: [██████████] 100%

## Performance Metrics

**Velocity:**

- Total plans completed: 42 (v0.1: 14 · v0.2: 7 · v0.3: 5 · v0.4: 16)

**By Milestone:**

| Milestone | Phases | Plans | Commits | Span |
|-----------|--------|-------|---------|------|
| v0.1 Initial Release | 4 | 14 | 127 | 7 days |
| v0.2 Declarative Rule Model | 3 | 7 | ~38 | 7 days |
| v0.3 Economy Items & Exceptional Gear | 3 | 5 | 54 | 15 days |
| v0.4 Overlay Filter Editor Rework | 5 | 16 | 146 | 14 days |

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

## Accumulated Context

### Roadmap Evolution

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

Last session: 2026-10-12T23:53:21.783Z
Stopped at: Completed 17-01-PLAN.md
Resume file: None

**Next step:** `/gsd-plan-phase 16` (Lossless Filter Parser). Phase 17 (Config-Driven Styles) is
independent and can run in parallel.

**Planned Phase:** 16 (Lossless Filter Parser) — 4 plans — 2026-10-12T22:08:51.700Z
